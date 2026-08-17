use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::services::login;
use crate::AppState;

#[derive(Deserialize)]
pub struct SetPasswordRequest {
    pub password: String,
}

/// Sets the caller's own (real user's) optional password. Refused while
/// impersonating — it would silently change the target's credential.
pub async fn set_my_password(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<SetPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    login::set_password(&state.db, auth.real_user.id, &body.password).await?;
    Ok(Json(json!({ "message": "password updated" })))
}
