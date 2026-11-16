use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::handlers::UserDto;
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::users as user_service;
use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateIdentityRequest {
    pub first_name: String,
    pub last_name: String,
}

/// Sets the caller's own first/last name (bootstrap accounts start with a
/// name derived from their email; admin typos happen). Refused while
/// impersonating — user-global identity data.
pub async fn set_my_identity(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<UpdateIdentityRequest>,
) -> Result<Json<UserDto>, AppError> {
    if auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    let before = format!("{} {}", auth.real_user.first_name, auth.real_user.last_name);
    let updated = user_service::update_names(
        &state.db,
        auth.real_user.clone(),
        &body.first_name,
        &body.last_name,
    )
    .await?;
    audit::record(
        &state.db,
        Actor::from(&auth),
        None,
        "user.identity.renamed",
        "user",
        Some(updated.id),
        json!({ "from": before, "to": format!("{} {}", updated.first_name, updated.last_name) }),
    )
    .await?;
    Ok(Json(UserDto::from(&updated)))
}
