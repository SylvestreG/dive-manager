use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::identity::extractors::{ClientIp, CurrentUser};
use crate::identity::handlers::{SessionResponse, UserDto};
use crate::identity::services::{audit, audit::Actor, login, sessions};
use crate::AppState;

#[derive(Deserialize)]
pub struct MagicLinkRequest {
    pub email: String,
}

/// Always answers the same generic message (no user enumeration).
pub async fn request_magic_link(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(body): Json<MagicLinkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    login::request_magic_link(&state, &body.email, &ip).await?;
    Ok(Json(json!({
        "message": "if this email is known, a login link has been sent"
    })))
}

#[derive(Deserialize)]
pub struct ConsumeMagicLinkRequest {
    pub token: String,
}

pub async fn consume_magic_link(
    State(state): State<AppState>,
    Json(body): Json<ConsumeMagicLinkRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let (token, user) = login::consume_magic_link(&state, &body.token).await?;
    Ok(Json(SessionResponse {
        token,
        user: UserDto::from(&user),
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    sessions::revoke_session(&state.db, auth.session.id).await?;
    Ok(Json(json!({ "message": "logged out" })))
}

pub async fn logout_all(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    sessions::revoke_all_for_user(&state.db, auth.real_user.id).await?;
    Ok(Json(json!({ "message": "all sessions revoked" })))
}

/// Ends impersonation: revokes the impersonation session and returns a fresh
/// session for the real user.
pub async fn stop_impersonation(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<SessionResponse>, AppError> {
    let Some(target_id) = auth.session.impersonated_user_id else {
        return Err(AppError::Validation("not impersonating".to_string()));
    };
    let txn = sea_orm::TransactionTrait::begin(&state.db).await?;
    sessions::revoke_session(&txn, auth.session.id).await?;
    let (token, _) =
        sessions::create_session(&txn, &state.config, auth.real_user.id, None, None).await?;
    audit::record(
        &txn,
        Actor {
            user_id: auth.real_user.id,
            impersonated_user_id: Some(target_id),
        },
        auth.session.impersonated_club_id,
        "impersonation.stopped",
        "user",
        Some(target_id),
        json!({}),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(SessionResponse {
        token,
        user: UserDto::from(&auth.real_user),
    }))
}
