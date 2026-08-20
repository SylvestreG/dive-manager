//! Club-visibility gates over user-global data, shared by every module that
//! serves member data through a club route (users, competencies, …).

use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::services::users as user_service;
use crate::tenancy::context::Tenant;
use crate::tenancy::permissions::Permission;
use crate::tenancy::services::memberships;
use crate::AppState;

/// Read access to the club's member data: the `club.members.view` grant, OR
/// being a CERTIFIED instructor (derived E1+ from held certifications) who is
/// an active member of this club — ADR-0007 amendment (product owner,
/// 2026-08-18): "un encadrant voit les plongeurs de son club". Read-only:
/// writes keep requiring their own permission/gates. The stagiaire MF1 status
/// grants nothing here (decision Q5: no application rights) — hence
/// `teaching_certified`, which is also age-independent (any reference
/// birthdate works when the profile has none).
pub async fn require_members_view(state: &AppState, tenant: &Tenant) -> Result<(), AppError> {
    if tenant.require(Permission::ClubMembersView).is_ok() {
        return Ok(());
    }
    if tenant.membership.is_some() {
        let held =
            crate::users::handlers::ffessm_certifications(&state.db, tenant.auth.effective_user.id)
                .await?;
        let person = ffessm::Person {
            certifications: &held,
            statuses: &[],
            birthdate: chrono::NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"),
            handisub: false,
        };
        let capacity = ffessm::supervision(&person, chrono::Utc::now().date_naive());
        if capacity.teaching_certified.is_some() {
            return Ok(());
        }
    }
    Err(AppError::Forbidden(
        "missing permission: club.members.view".to_string(),
    ))
}

/// The tenant gate for user-global data: 404 unless `user_id` is an ACTIVE
/// member of the tenant club (never reveal that the user exists elsewhere).
pub async fn ensure_member(
    state: &AppState,
    tenant: &Tenant,
    user_id: Uuid,
) -> Result<users::Model, AppError> {
    let not_found = || AppError::NotFound("member not found".to_string());
    memberships::find_active(&state.db, tenant.club.id, user_id)
        .await?
        .ok_or_else(not_found)?;
    user_service::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(not_found)
}
