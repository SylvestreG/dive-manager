//! OIDC endpoints (ADR-0009). The start and callback endpoints are BROWSER
//! navigations: they always answer with a redirect to a frontend page,
//! never with JSON — errors become `/login?error=…`.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Redirect;
use axum::{Form, Json};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
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

/// GET /auth/oidc/{provider}/start → 302 to the provider.
pub async fn start(State(state): State<AppState>, Path(provider): Path<String>) -> Redirect {
    let Some(provider) = Provider::parse(&provider) else {
        return frontend(&state, "/login?error=sso");
    };
    match oidc::start(&state, provider, None).await {
        Ok(url) => Redirect::to(&url),
        Err(error) => {
            tracing::warn!(?error, "OIDC start failed");
            frontend(&state, "/login?error=sso")
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
) -> Redirect {
    let Some(provider) = Provider::parse(&provider) else {
        return frontend(&state, "/login?error=sso");
    };
    if params.error.is_some() {
        return frontend(&state, "/login?error=sso_cancelled");
    }
    let (Some(code), Some(flow_state)) = (params.code, params.state) else {
        return frontend(&state, "/login?error=sso");
    };
    let outcome =
        match oidc::callback(&state, provider, &code, &flow_state, device_label(&headers)).await {
            Ok(outcome) => outcome,
            Err(error) => {
                tracing::warn!(?error, provider = provider.as_str(), "OIDC callback failed");
                CallbackOutcome::Failed
            }
        };
    let path = match outcome {
        CallbackOutcome::LoggedIn { exchange_code } => format!("/auth/done?code={exchange_code}"),
        CallbackOutcome::ConfirmationSent => "/auth/check-email".to_string(),
        CallbackOutcome::UnknownAccount => "/auth/unknown".to_string(),
        CallbackOutcome::Linked => format!("/profile?linked={}", provider.as_str()),
        CallbackOutcome::AlreadyUsed => "/profile?link_error=already_used".to_string(),
        CallbackOutcome::Failed => "/login?error=sso".to_string(),
    };
    frontend(&state, &path)
}

/// GET callback (Google, Microsoft).
pub async fn callback_get(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Query(params): Query<CallbackParams>,
) -> Redirect {
    finish(state, provider, params, headers).await
}

/// POST callback (Apple `response_mode=form_post`).
pub async fn callback_post(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Form(params): Form<CallbackParams>,
) -> Redirect {
    finish(state, provider, params, headers).await
}

#[derive(Deserialize)]
pub struct ExchangeRequest {
    pub code: String,
}

/// POST /auth/exchange — one-time code → session (the code reached the page
/// through a redirect URL; the session token never does).
pub async fn exchange(
    State(state): State<AppState>,
    Json(body): Json<ExchangeRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let (token, user) = oidc::exchange(&state, &body.code).await?;
    Ok(Json(SessionResponse {
        token,
        user: UserDto::from(&user),
    }))
}
