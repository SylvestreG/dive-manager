//! OIDC endpoints (ADR-0009). The start and callback endpoints are BROWSER
//! navigations: they always answer with a redirect to a frontend page,
//! never with JSON — errors become `/login?error=…`.

use std::time::Duration as StdDuration;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{AppendHeaders, IntoResponse, Redirect, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::errors::AppError;
use crate::identity::extractors::ClientIp;
use crate::identity::handlers::auth::device_label;
use crate::identity::handlers::{SessionResponse, UserDto};
use crate::identity::services::oidc::{self, CallbackOutcome, Provider};
use crate::AppState;

#[derive(Serialize)]
pub struct ProviderDto {
    pub id: Provider,
    pub label: &'static str,
}

/// Providers configured on this instance (login buttons).
pub async fn providers(State(state): State<AppState>) -> Json<Vec<ProviderDto>> {
    Json(
        oidc::configured(&state.config)
            .into_iter()
            .map(|p| ProviderDto {
                id: p,
                label: p.label(),
            })
            .collect(),
    )
}

fn frontend(state: &AppState, path: &str) -> Redirect {
    Redirect::to(&format!("{}{path}", state.config.base_url))
}

// ---------- browser binding cookie ----------
//
// Set when a flow starts, required at the callback AND at the code
// exchange: a flow can only be finished by the browser that started it
// (no login CSRF, no link flow completed by a victim). SameSite=None so it
// also accompanies Apple's cross-site form_post callback; it authenticates
// nothing by itself (a random value whose hash is bound to one flow).

fn secure(config: &Config) -> bool {
    config.base_url.starts_with("https://")
}

fn cookie_name(config: &Config) -> &'static str {
    if secure(config) {
        "__Host-dm_oidc"
    } else {
        "dm_oidc"
    }
}

fn cookie_attributes(config: &Config) -> &'static str {
    if secure(config) {
        "Path=/; HttpOnly; Secure; SameSite=None"
    } else {
        // Plain-http dev: SameSite=None requires Secure.
        "Path=/; HttpOnly; SameSite=Lax"
    }
}

pub(crate) fn set_binding_cookie(config: &Config, value: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}={value}; Max-Age=600; {}",
        cookie_name(config),
        cookie_attributes(config)
    ))
    .expect("cookie header")
}

fn clear_binding_cookie(config: &Config) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}=; Max-Age=0; {}",
        cookie_name(config),
        cookie_attributes(config)
    ))
    .expect("cookie header")
}

fn read_binding(config: &Config, headers: &HeaderMap) -> Option<String> {
    let name = cookie_name(config);
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_string())
}

/// GET /auth/oidc/{provider}/start → 303 to the provider, setting the
/// browser-binding cookie. Rate limited per IP (design §11).
pub async fn start(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(provider): Path<String>,
) -> Response {
    let Some(provider) = Provider::parse(&provider) else {
        return frontend(&state, "/login?error=sso").into_response();
    };
    if !state.rate.check(
        &format!("oidc-start:{ip}"),
        30,
        StdDuration::from_secs(15 * 60),
    ) {
        return frontend(&state, "/login?error=rate_limited").into_response();
    }
    match oidc::start(&state, provider, None).await {
        Ok(flow) => (
            AppendHeaders([(
                header::SET_COOKIE,
                set_binding_cookie(&state.config, &flow.browser_binding),
            )]),
            Redirect::to(&flow.authorize_url),
        )
            .into_response(),
        Err(error) => {
            tracing::warn!(?error, "OIDC start failed");
            frontend(&state, "/login?error=sso").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    /// Set by the provider when the user cancels or on provider errors.
    pub error: Option<String>,
}

async fn finish(
    state: AppState,
    provider: String,
    params: CallbackParams,
    headers: HeaderMap,
) -> Response {
    let clear = AppendHeaders([(header::SET_COOKIE, clear_binding_cookie(&state.config))]);
    let Some(provider) = Provider::parse(&provider) else {
        return (clear, frontend(&state, "/login?error=sso")).into_response();
    };
    if params.error.is_some() {
        return (clear, frontend(&state, "/login?error=sso_cancelled")).into_response();
    }
    let (Some(code), Some(flow_state)) = (params.code, params.state) else {
        return (clear, frontend(&state, "/login?error=sso")).into_response();
    };
    let binding = read_binding(&state.config, &headers);
    let outcome = match oidc::callback(
        &state,
        provider,
        &code,
        &flow_state,
        binding.as_deref(),
        device_label(&headers),
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::warn!(?error, provider = provider.as_str(), "OIDC callback failed");
            CallbackOutcome::Failed
        }
    };
    let path = match outcome {
        CallbackOutcome::LoggedIn { exchange_code } => {
            // Keep the binding cookie: the exchange needs it (then clears it).
            return frontend(&state, &format!("/auth/done?code={exchange_code}")).into_response();
        }
        CallbackOutcome::ConfirmationSent => "/auth/check-email".to_string(),
        CallbackOutcome::UnknownAccount => "/auth/unknown".to_string(),
        CallbackOutcome::Linked => format!("/profile?linked={}", provider.as_str()),
        CallbackOutcome::AlreadyUsed => "/profile?link_error=already_used".to_string(),
        CallbackOutcome::Failed => "/login?error=sso".to_string(),
    };
    (clear, frontend(&state, &path)).into_response()
}

/// GET callback (Google, Microsoft).
pub async fn callback_get(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Query(params): Query<CallbackParams>,
) -> Response {
    finish(state, provider, params, headers).await
}

/// POST callback (Apple `response_mode=form_post`).
pub async fn callback_post(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Form(params): Form<CallbackParams>,
) -> Response {
    finish(state, provider, params, headers).await
}

#[derive(Deserialize)]
pub struct ExchangeRequest {
    pub code: String,
}

/// POST /auth/exchange — one-time code → session, only from the browser
/// that started the flow (binding cookie). The session token never travels
/// in a URL.
pub async fn exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExchangeRequest>,
) -> Result<Response, AppError> {
    let binding = read_binding(&state.config, &headers);
    let (token, user) = oidc::exchange(&state, &body.code, binding.as_deref()).await?;
    Ok((
        AppendHeaders([(header::SET_COOKIE, clear_binding_cookie(&state.config))]),
        Json(SessionResponse {
            token,
            user: UserDto::from(&user),
        }),
    )
        .into_response())
}
