use std::collections::{HashMap, HashSet};

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use axum::RequestPartsExt;
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::services::audit::Actor;
use crate::identity::services::sessions::AuthSession;
use crate::tenancy::entities::{club_role_permissions, club_roles, clubs, memberships, prelude::*};
use crate::tenancy::permissions::Permission;
use crate::AppState;

/// Tenant context: the club addressed by the URL, verified against the
/// effective user's membership (or global-admin status). Every club-scoped
/// handler receives this; club-owned queries must filter on `club.id`.
pub struct Tenant {
    pub club: clubs::Model,
    pub auth: AuthSession,
    pub membership: Option<memberships::Model>,
    pub role: Option<club_roles::Model>,
    pub permissions: HashSet<Permission>,
    /// True when access comes from global-admin status instead of membership.
    pub via_global_admin: bool,
}

impl Tenant {
    pub fn require(&self, permission: Permission) -> Result<(), AppError> {
        if self.permissions.contains(&permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "missing permission: {permission}"
            )))
        }
    }

    /// Permission check that additionally refuses impersonating principals —
    /// for destructive/authorization-sensitive actions (design §5).
    pub fn require_real(&self, permission: Permission) -> Result<(), AppError> {
        if self.auth.is_impersonating() {
            return Err(AppError::Forbidden(
                "action not allowed while impersonating".to_string(),
            ));
        }
        self.require(permission)
    }

    pub fn actor(&self) -> Actor {
        Actor::from(&self.auth)
    }
}

impl FromRequestParts<AppState> for Tenant {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let CurrentUser(auth) = CurrentUser::from_request_parts(parts, state).await?;

        let params: Path<HashMap<String, String>> = parts.extract().await.map_err(|_| {
            tracing::error!("tenant extractor used on a route without path parameters");
            AppError::Internal
        })?;
        let slug = params.get("club_slug").cloned().ok_or_else(|| {
            tracing::error!("tenant extractor used on a route without {{club_slug}}");
            AppError::Internal
        })?;

        let club = Clubs::find()
            .filter(clubs::Column::Slug.eq(slug.to_lowercase()))
            .filter(clubs::Column::Status.eq(clubs::STATUS_ACTIVE))
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("club not found".to_string()))?;

        // A club-scoped impersonation session never crosses club boundaries,
        // even into other clubs the impersonated user belongs to.
        if let Some(scope_club_id) = auth.session.impersonated_club_id {
            if scope_club_id != club.id {
                return Err(AppError::Forbidden(
                    "impersonation is limited to its club".to_string(),
                ));
            }
        }

        let membership_with_role = Memberships::find()
            .filter(memberships::Column::ClubId.eq(club.id))
            .filter(memberships::Column::UserId.eq(auth.effective_user.id))
            .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE))
            .find_also_related(ClubRoles)
            .one(&state.db)
            .await?;

        match membership_with_role {
            Some((membership, Some(role))) => {
                let permissions = load_role_permissions(&state.db, &role).await?;
                Ok(Tenant {
                    club,
                    auth,
                    membership: Some(membership),
                    role: Some(role),
                    permissions,
                    via_global_admin: false,
                })
            }
            Some((_, None)) => {
                tracing::error!("membership without role (FK should prevent this)");
                Err(AppError::Internal)
            }
            None => {
                if auth.real_user.is_global_admin && !auth.is_impersonating() {
                    Ok(Tenant {
                        club,
                        auth,
                        membership: None,
                        role: None,
                        permissions: Permission::ALL.into_iter().collect(),
                        via_global_admin: true,
                    })
                } else {
                    Err(AppError::Forbidden("not a member of this club".to_string()))
                }
            }
        }
    }
}

pub async fn load_role_permissions(
    db: &DatabaseConnection,
    role: &club_roles::Model,
) -> Result<HashSet<Permission>, AppError> {
    if role.is_admin {
        return Ok(Permission::ALL.into_iter().collect());
    }
    let rows = ClubRolePermissions::find()
        .filter(club_role_permissions::Column::RoleId.eq(role.id))
        .all(db)
        .await?;
    // Unknown strings are ignored, never granted (fail closed).
    Ok(rows
        .iter()
        .filter_map(|row| Permission::parse(&row.permission))
        .collect())
}

/// Resolves the REAL user's permissions in a club — used by impersonation,
/// whose authorization must never be evaluated against the impersonated
/// target (foundation design §5, switch semantics).
pub async fn real_user_club_permissions(
    db: &DatabaseConnection,
    club_id: Uuid,
    auth: &AuthSession,
) -> Result<HashSet<Permission>, AppError> {
    if auth.real_user.is_global_admin {
        return Ok(Permission::ALL.into_iter().collect());
    }
    let membership_with_role = Memberships::find()
        .filter(memberships::Column::ClubId.eq(club_id))
        .filter(memberships::Column::UserId.eq(auth.real_user.id))
        .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE))
        .find_also_related(ClubRoles)
        .one(db)
        .await?;
    match membership_with_role {
        Some((_, Some(role))) => load_role_permissions(db, &role).await,
        _ => Ok(HashSet::new()),
    }
}
