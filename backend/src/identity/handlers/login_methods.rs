//! Self-service login methods and devices (profile page, ADR-0009 R10–R12,
//! R29). All writes refuse impersonating principals: they act on the REAL
//! user's credentials.

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::handlers::oidc::set_binding_cookie;
use crate::identity::services::login;
use crate::identity::services::oidc::{self, Provider};
use crate::identity::services::sessions::{self, AuthSession};
use crate::AppState;

fn refuse_impersonation(auth: &AuthSession) -> Result<(), AppError> {
    if auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    Ok(())
}

#[derive(Serialize)]
pub struct IdentityDto {
    pub id: Uuid,
    pub provider: String,
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub last_used_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Serialize)]
pub struct LoginMethodsResponse {
    /// Magic links go here; None = no email login.
    pub email: Option<String>,
    pub email_verified: bool,
    pub identities: Vec<IdentityDto>,
    /// Providers configured on this instance (link buttons).
    pub available_providers: Vec<Provider>,
}

pub async fn get_login_methods(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<LoginMethodsResponse>, AppError> {
    let user = &auth.real_user;
    let identities = oidc::list_for_user(&state.db, user.id).await?;
    Ok(Json(LoginMethodsResponse {
        email: user.email.clone(),
        email_verified: user.email_verified_at.is_some(),
        identities: identities
            .into_iter()
            .map(|i| IdentityDto {
                id: i.id,
                provider: i.provider,
                email: i.email_at_link,
                created_at: i.created_at,
                last_used_at: i.last_used_at,
            })
            .collect(),
        available_providers: oidc::configured(&state.config),
    }))
}

/// Starts a link flow bound to the caller AND to this browser (cookie set
/// on this same-origin response); the page then navigates to `url`.
pub async fn start_link(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(provider): Path<String>,
) -> Result<Response, AppError> {
    refuse_impersonation(&auth)?;
    let provider = Provider::parse(&provider)
        .ok_or_else(|| AppError::NotFound("unknown login provider".to_string()))?;
    if !state.rate.check(
        &format!("oidc-link:{}", auth.real_user.id),
        10,
        std::time::Duration::from_secs(15 * 60),
    ) {
        return Err(AppError::TooManyRequests);
    }
    let flow = oidc::start(&state, provider, Some(auth.real_user.id)).await?;
    Ok((
        AppendHeaders([(
            header::SET_COOKIE,
            set_binding_cookie(&state.config, &flow.browser_binding),
        )]),
        Json(json!({ "url": flow.authorize_url })),
    )
        .into_response())
}

pub async fn unlink(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(identity_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    refuse_impersonation(&auth)?;
    oidc::unlink(&state, auth.real_user.id, identity_id).await?;
    Ok(Json(json!({ "message": "login method removed" })))
}

#[derive(Deserialize)]
pub struct ChangeEmailRequest {
    pub email: String,
}

/// R12: emails a confirmation link to the NEW address; nothing changes
/// until it is clicked.
pub async fn change_email(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<ChangeEmailRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    refuse_impersonation(&auth)?;
    login::request_email_change(&state, &auth.real_user, &body.email).await?;
    Ok(Json(
        json!({ "message": "confirmation sent to the new address" }),
    ))
}

#[derive(Serialize)]
pub struct DeviceDto {
    pub id: Uuid,
    pub label: Option<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub last_seen_at: chrono::DateTime<chrono::FixedOffset>,
    pub current: bool,
}

pub async fn list_devices(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<Vec<DeviceDto>>, AppError> {
    let current = auth.session.id;
    let rows = sessions::list_active_for_user(&state.db, auth.real_user.id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|s| DeviceDto {
                id: s.id,
                label: s.client_label,
                created_at: s.created_at,
                last_seen_at: s.last_seen_at,
                current: s.id == current,
            })
            .collect(),
    ))
}

pub async fn revoke_device(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    refuse_impersonation(&auth)?;
    sessions::revoke_own(&state.db, auth.real_user.id, session_id).await?;
    Ok(Json(json!({ "message": "device logged out" })))
}
