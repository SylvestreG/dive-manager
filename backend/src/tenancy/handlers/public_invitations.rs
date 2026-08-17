use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::identity::handlers::{SessionResponse, UserDto};
use crate::tenancy::services::invitations;
use crate::AppState;

#[derive(Serialize)]
pub struct InvitationPreviewDto {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    /// None = global-administrator invitation.
    pub club_name: Option<String>,
    pub role_name: Option<String>,
    pub expires_at: sea_orm::prelude::DateTimeWithTimeZone,
}

pub async fn preview(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<InvitationPreviewDto>, AppError> {
    let preview = invitations::preview_invitation(&state.db, &token).await?;
    Ok(Json(InvitationPreviewDto {
        email: preview.invitation.email,
        first_name: preview.invitation.first_name,
        last_name: preview.invitation.last_name,
        club_name: preview.club_name,
        role_name: preview.role_name,
        expires_at: preview.invitation.expires_at,
    }))
}

#[derive(Deserialize)]
pub struct AcceptInvitationRequest {
    /// Optional: set a password while activating (ADR-0004, optional
    /// credential; magic links remain available either way).
    pub password: Option<String>,
}

pub async fn accept(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(body): Json<AcceptInvitationRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let (session_token, user) =
        invitations::accept_invitation(&state, &token, body.password.as_deref()).await?;
    Ok(Json(SessionResponse {
        token: session_token,
        user: UserDto::from(&user),
    }))
}
