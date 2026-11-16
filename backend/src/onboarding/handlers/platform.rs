//! Platform intake endpoints — real (non-impersonating) global admins only.

use axum::extract::{Path, State};
use axum::Json;
use sea_orm::TransactionTrait;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::services::audit::Actor;
use crate::onboarding::handlers::members::{AddMemberResponse, IntakeUserDto};
use crate::onboarding::services::intake::{self, NewMember};
use crate::tenancy::handlers::platform::require_real_global_admin;
use crate::tenancy::services::roles;
use crate::users::services::profiles::IntakeFields;
use crate::AppState;

#[derive(Deserialize)]
pub struct AddClubAdminRequest {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}

/// Makes a person an Administrateur of the club: creates the account or
/// reuses the one matching the email, then sends a welcome email. Replaces
/// the former club-admin invitation (ADR-0009). An email is mandatory here:
/// a club admin must be able to log in by magic link right away.
pub async fn add_club_admin(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(club_id): Path<Uuid>,
    Json(body): Json<AddClubAdminRequest>,
) -> Result<Json<AddMemberResponse>, AppError> {
    require_real_global_admin(&auth)?;
    if body.email.trim().is_empty() {
        return Err(AppError::Validation("email is required".to_string()));
    }
    let club = roles::active_club(&state.db, club_id).await?;
    let role = roles::admin_role(&state.db, club.id).await?;

    let txn = state.db.begin().await?;
    let result = intake::add_member(
        &txn,
        Actor::from(&auth),
        &club,
        &role,
        NewMember {
            first_name: body.first_name,
            last_name: body.last_name,
            email: Some(body.email),
            profile: IntakeFields::default(),
            // A platform admin names a club administrator: an existing
            // account under another name is still that account.
            confirm_existing: true,
        },
    )
    .await?;
    txn.commit().await?;
    intake::send_welcome(&state, &result.user, Some(&club.name)).await;

    Ok(Json(AddMemberResponse {
        user: IntakeUserDto::from(&result.user),
        membership_id: result.membership_id,
        role_id: role.id,
        role_name: role.name,
        is_admin: role.is_admin,
        outcome: result.outcome,
        diffs: result.diffs,
    }))
}
