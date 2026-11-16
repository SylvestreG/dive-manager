//! Club role read services exposed to other modules (module-boundary rule).

use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::tenancy::entities::{club_roles, prelude::*};

/// A role of `club_id` — 404 for an unknown id or another club's role
/// (tenant isolation: foreign roles are invisible, not forbidden).
pub async fn find_in_club<C: ConnectionTrait>(
    conn: &C,
    club_id: Uuid,
    role_id: Uuid,
) -> Result<club_roles::Model, AppError> {
    ClubRoles::find_by_id(role_id)
        .filter(club_roles::Column::ClubId.eq(club_id))
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("role not found".to_string()))
}

/// The club's administrator role (seed invariant: exactly one per club).
pub async fn admin_role<C: ConnectionTrait>(
    conn: &C,
    club_id: Uuid,
) -> Result<club_roles::Model, AppError> {
    ClubRoles::find()
        .filter(club_roles::Column::ClubId.eq(club_id))
        .filter(club_roles::Column::IsAdmin.eq(true))
        .one(conn)
        .await?
        .ok_or_else(|| {
            tracing::error!(%club_id, "club has no admin role (seed invariant)");
            AppError::Internal
        })
}

/// An active club by id (platform routes address clubs by id).
pub async fn active_club<C: ConnectionTrait>(
    conn: &C,
    club_id: Uuid,
) -> Result<crate::tenancy::entities::clubs::Model, AppError> {
    use crate::tenancy::entities::clubs;
    Clubs::find_by_id(club_id)
        .filter(clubs::Column::Status.eq(clubs::STATUS_ACTIVE))
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("club not found".to_string()))
}
