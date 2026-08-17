//! `/me` composition endpoint. Lives in `tenancy` because it joins the
//! authenticated identity with club memberships — the dependency direction is
//! tenancy → identity, never the reverse (module-boundary rule).

use axum::extract::State;
use axum::Json;
use sea_orm::*;
use serde::Serialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::handlers::UserDto;
use crate::tenancy::entities::{clubs, memberships, prelude::*};
use crate::tenancy::permissions::Permission;
use crate::AppState;

#[derive(Serialize)]
pub struct MeMembershipDto {
    pub club_id: Uuid,
    pub club_slug: String,
    pub club_name: String,
    pub role_name: String,
    pub is_club_admin: bool,
    pub permissions: Vec<String>,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user: UserDto,
    pub impersonating: bool,
    /// The acting administrator, present only while impersonating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_user: Option<UserDto>,
    /// Whether the REAL user has a password credential (the profile's
    /// password form always targets the real user).
    pub has_password: bool,
    pub memberships: Vec<MeMembershipDto>,
}

pub async fn get_me(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<MeResponse>, AppError> {
    let mut membership_query = Memberships::find()
        .filter(memberships::Column::UserId.eq(auth.effective_user.id))
        .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE));
    // A club-scoped impersonation only reveals the target's life in that club.
    if let Some(scope_club_id) = auth.session.impersonated_club_id {
        membership_query = membership_query.filter(memberships::Column::ClubId.eq(scope_club_id));
    }
    let membership_rows = membership_query
        .find_also_related(Clubs)
        .all(&state.db)
        .await?;

    let role_ids: Vec<Uuid> = membership_rows.iter().map(|(m, _)| m.role_id).collect();
    let roles = ClubRoles::find()
        .filter(crate::tenancy::entities::club_roles::Column::Id.is_in(role_ids.clone()))
        .all(&state.db)
        .await?;
    let permission_rows = ClubRolePermissions::find()
        .filter(crate::tenancy::entities::club_role_permissions::Column::RoleId.is_in(role_ids))
        .all(&state.db)
        .await?;

    let mut memberships_dto = Vec::with_capacity(membership_rows.len());
    for (membership, club) in &membership_rows {
        let Some(club) = club else { continue };
        if club.status != clubs::STATUS_ACTIVE {
            continue;
        }
        let Some(role) = roles.iter().find(|r| r.id == membership.role_id) else {
            continue;
        };
        let permissions: Vec<String> = if role.is_admin {
            Permission::ALL
                .iter()
                .map(|p| p.as_str().to_string())
                .collect()
        } else {
            permission_rows
                .iter()
                .filter(|row| row.role_id == role.id)
                .filter_map(|row| Permission::parse(&row.permission))
                .map(|p| p.as_str().to_string())
                .collect()
        };
        memberships_dto.push(MeMembershipDto {
            club_id: club.id,
            club_slug: club.slug.clone(),
            club_name: club.name.clone(),
            role_name: role.name.clone(),
            is_club_admin: role.is_admin,
            permissions,
        });
    }

    let has_password =
        crate::identity::services::login::user_has_password(&state.db, auth.real_user.id).await?;

    Ok(Json(MeResponse {
        user: UserDto::from(&auth.effective_user),
        impersonating: auth.is_impersonating(),
        real_user: auth
            .is_impersonating()
            .then(|| UserDto::from(&auth.real_user)),
        has_password,
        memberships: memberships_dto,
    }))
}
