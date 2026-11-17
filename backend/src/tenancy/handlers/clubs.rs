//! Club-scoped endpoints. Every handler receives the [`Tenant`] context;
//! every query on club-owned tables filters on `tenant.club.id`.

use axum::extract::{Path, Query, State};
use axum::Json;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::{audit_log, prelude::AuditLog, prelude::Users, users};
use crate::identity::handlers::{SessionResponse, UserDto};
use crate::identity::services::{audit, sessions};
use crate::pagination::PageParams;
use crate::tenancy::context::{real_user_club_permissions, Tenant};
use crate::tenancy::entities::{
    club_modules, club_role_permissions, club_roles, clubs, memberships, prelude::*,
};
use crate::tenancy::permissions::Permission;
use crate::tenancy::services::seed;
use crate::{now, AppState};

// ---------- club settings ----------

#[derive(Serialize)]
pub struct ClubContextResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub my_role: Option<String>,
    pub permissions: Vec<String>,
    pub via_global_admin: bool,
}

pub async fn get_club(tenant: Tenant) -> Json<ClubContextResponse> {
    Json(ClubContextResponse {
        id: tenant.club.id,
        name: tenant.club.name.clone(),
        slug: tenant.club.slug.clone(),
        my_role: tenant.role.as_ref().map(|r| r.name.clone()),
        permissions: tenant
            .permissions
            .iter()
            .map(|p| p.as_str().to_string())
            .collect(),
        via_global_admin: tenant.via_global_admin,
    })
}

#[derive(Deserialize)]
pub struct UpdateClubRequest {
    pub name: String,
}

pub async fn update_club(
    State(state): State<AppState>,
    tenant: Tenant,
    Json(body): Json<UpdateClubRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tenant.require(Permission::ClubManage)?;
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("name cannot be empty".to_string()));
    }
    let mut active: clubs::ActiveModel = tenant.club.clone().into();
    active.name = Set(name.clone());
    active.updated_at = Set(now());

    // Mutation and audit commit together (review M6).
    let txn = state.db.begin().await?;
    active.update(&txn).await?;
    audit::record(
        &txn,
        tenant.actor(),
        Some(tenant.club.id),
        "club.updated",
        "club",
        Some(tenant.club.id),
        json!({ "old_name": tenant.club.name, "new_name": name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "club updated" })))
}

// ---------- modules ----------

#[derive(Serialize)]
pub struct ModuleDto {
    pub module: String,
    pub enabled: bool,
}

pub async fn list_modules(
    State(state): State<AppState>,
    tenant: Tenant,
) -> Result<Json<Vec<ModuleDto>>, AppError> {
    tenant.require(Permission::ClubManage)?;
    let rows = ClubModules::find()
        .filter(club_modules::Column::ClubId.eq(tenant.club.id))
        .order_by_asc(club_modules::Column::Module)
        .all(&state.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ModuleDto {
                module: r.module,
                enabled: r.enabled,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SetModuleRequest {
    pub module: String,
    pub enabled: bool,
}

pub async fn set_module(
    State(state): State<AppState>,
    tenant: Tenant,
    Json(body): Json<SetModuleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tenant.require(Permission::ClubManage)?;
    if !seed::MODULES.contains(&body.module.as_str()) {
        return Err(AppError::Validation("unknown module".to_string()));
    }
    let existing = ClubModules::find_by_id((tenant.club.id, body.module.clone()))
        .one(&state.db)
        .await?;
    match existing {
        Some(row) => {
            let mut active: club_modules::ActiveModel = row.into();
            active.enabled = Set(body.enabled);
            active.update(&state.db).await?;
        }
        None => {
            club_modules::ActiveModel {
                club_id: Set(tenant.club.id),
                module: Set(body.module.clone()),
                enabled: Set(body.enabled),
            }
            .insert(&state.db)
            .await?;
        }
    }
    audit::record(
        &state.db,
        tenant.actor(),
        Some(tenant.club.id),
        "club.module_toggled",
        "club_module",
        None,
        json!({ "module": body.module, "enabled": body.enabled }),
    )
    .await?;
    Ok(Json(json!({ "message": "module updated" })))
}

// ---------- roles ----------

#[derive(Serialize)]
pub struct RoleDto {
    pub id: Uuid,
    pub name: String,
    pub is_admin: bool,
    /// Admin roles have every permission implicitly.
    pub permissions: Vec<String>,
}

pub async fn list_roles(
    State(state): State<AppState>,
    tenant: Tenant,
) -> Result<Json<Vec<RoleDto>>, AppError> {
    // Reading roles serves the member intake forms too; editing them
    // (set_role_permissions) stays locked behind club.roles.manage.
    tenant.require_any(&[
        Permission::ClubRolesManage,
        Permission::ClubMembersView,
        Permission::ClubIntakeManage,
    ])?;
    let roles = ClubRoles::find()
        .filter(club_roles::Column::ClubId.eq(tenant.club.id))
        .order_by_asc(club_roles::Column::Name)
        .all(&state.db)
        .await?;
    let role_ids: Vec<Uuid> = roles.iter().map(|r| r.id).collect();
    let permission_rows = ClubRolePermissions::find()
        .filter(club_role_permissions::Column::RoleId.is_in(role_ids))
        .all(&state.db)
        .await?;
    Ok(Json(
        roles
            .into_iter()
            .map(|role| {
                let permissions = if role.is_admin {
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
                RoleDto {
                    id: role.id,
                    name: role.name,
                    is_admin: role.is_admin,
                    permissions,
                }
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SetRolePermissionsRequest {
    pub permissions: Vec<String>,
}

pub async fn set_role_permissions(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, role_id)): Path<(String, Uuid)>,
    Json(body): Json<SetRolePermissionsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tenant.require_real(Permission::ClubRolesManage)?;
    let role = ClubRoles::find_by_id(role_id)
        .filter(club_roles::Column::ClubId.eq(tenant.club.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("role not found".to_string()))?;
    if role.is_admin {
        return Err(AppError::Validation(
            "the administrator role has every permission implicitly".to_string(),
        ));
    }
    let mut parsed = Vec::with_capacity(body.permissions.len());
    for raw in &body.permissions {
        let permission = Permission::parse(raw)
            .ok_or_else(|| AppError::Validation(format!("unknown permission: {raw}")))?;
        if !parsed.contains(&permission) {
            parsed.push(permission);
        }
    }

    let txn = state.db.begin().await?;
    ClubRolePermissions::delete_many()
        .filter(club_role_permissions::Column::RoleId.eq(role.id))
        .exec(&txn)
        .await?;
    if !parsed.is_empty() {
        club_role_permissions::Entity::insert_many(parsed.iter().map(|p| {
            club_role_permissions::ActiveModel {
                role_id: Set(role.id),
                permission: Set(p.as_str().to_string()),
            }
        }))
        .exec(&txn)
        .await?;
    }
    audit::record(
        &txn,
        tenant.actor(),
        Some(tenant.club.id),
        "role.permissions_updated",
        "club_role",
        Some(role.id),
        json!({ "permissions": parsed.iter().map(|p| p.as_str()).collect::<Vec<_>>() }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "permissions updated" })))
}

// ---------- memberships ----------

#[derive(Serialize)]
pub struct MembershipDto {
    pub id: Uuid,
    pub user: UserDto,
    pub role_id: Uuid,
    pub role_name: String,
    pub is_admin: bool,
    pub status: String,
}

#[derive(Deserialize)]
pub struct UpdateMembershipRequest {
    pub role_id: Option<Uuid>,
    pub status: Option<String>,
}

pub async fn update_membership(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, membership_id)): Path<(String, Uuid)>,
    Json(body): Json<UpdateMembershipRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tenant.require_real(Permission::ClubRolesManage)?;
    if let Some(status) = &body.status {
        if status != memberships::STATUS_ACTIVE && status != memberships::STATUS_SUSPENDED {
            return Err(AppError::Validation("invalid status".to_string()));
        }
    }

    // Everything happens in one transaction with the relevant membership rows
    // locked (FOR UPDATE): the last-admin guard would otherwise be a
    // check-then-act race between two concurrent demotions (review M1), and
    // the audit entry must commit with the mutation (review M6).
    let txn = state.db.begin().await?;

    let membership = Memberships::find_by_id(membership_id)
        .filter(memberships::Column::ClubId.eq(tenant.club.id))
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("membership not found".to_string()))?;

    let new_role = match body.role_id {
        Some(role_id) => Some(
            ClubRoles::find_by_id(role_id)
                .filter(club_roles::Column::ClubId.eq(tenant.club.id))
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::NotFound("role not found".to_string()))?,
        ),
        None => None,
    };

    // Lockout guard: a club must always keep one active administrator.
    let admin_role_ids: Vec<Uuid> = ClubRoles::find()
        .filter(club_roles::Column::ClubId.eq(tenant.club.id))
        .filter(club_roles::Column::IsAdmin.eq(true))
        .all(&txn)
        .await?
        .into_iter()
        .map(|r| r.id)
        .collect();
    let is_currently_active_admin = admin_role_ids.contains(&membership.role_id)
        && membership.status == memberships::STATUS_ACTIVE;
    let stays_active_admin = new_role
        .as_ref()
        .map(|r| r.is_admin)
        .unwrap_or(admin_role_ids.contains(&membership.role_id))
        && body.status.as_deref().unwrap_or(memberships::STATUS_ACTIVE)
            == memberships::STATUS_ACTIVE;
    if is_currently_active_admin && !stays_active_admin {
        let other_admins = Memberships::find()
            .filter(memberships::Column::ClubId.eq(tenant.club.id))
            .filter(memberships::Column::Id.ne(membership.id))
            .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE))
            .filter(memberships::Column::RoleId.is_in(admin_role_ids.clone()))
            .lock_exclusive()
            .all(&txn)
            .await?
            .len();
        if other_admins == 0 {
            return Err(AppError::Validation(
                "cannot remove the club's last administrator".to_string(),
            ));
        }
    }

    let membership_user_id = membership.user_id;
    let old_role_id = membership.role_id;
    let old_status = membership.status.clone();
    let mut active: memberships::ActiveModel = membership.into();
    if let Some(role) = &new_role {
        active.role_id = Set(role.id);
    }
    if let Some(status) = &body.status {
        active.status = Set(status.clone());
    }
    active.updated_at = Set(now());
    let updated = active.update(&txn).await?;

    audit::record(
        &txn,
        tenant.actor(),
        Some(tenant.club.id),
        "membership.updated",
        "membership",
        Some(membership_id),
        json!({
            "user_id": membership_user_id,
            "old_role_id": old_role_id,
            "new_role_id": updated.role_id,
            "old_status": old_status,
            "new_status": updated.status,
        }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "membership updated" })))
}

#[derive(Deserialize)]
pub struct ImpersonateRequest {
    pub user_id: Uuid,
}

/// Starts (or switches) impersonation. Authorization is evaluated against the
/// REAL user; the target must be an active member of this club and not a
/// global admin (foundation design §5).
pub async fn impersonate(
    State(state): State<AppState>,
    tenant: Tenant,
    Json(body): Json<ImpersonateRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let real_permissions =
        real_user_club_permissions(&state.db, tenant.club.id, &tenant.auth).await?;
    if !real_permissions.contains(&Permission::ClubImpersonate) {
        return Err(AppError::Forbidden(format!(
            "missing permission: {}",
            Permission::ClubImpersonate
        )));
    }
    if body.user_id == tenant.auth.real_user.id {
        return Err(AppError::Validation(
            "cannot impersonate yourself".to_string(),
        ));
    }

    Memberships::find()
        .filter(memberships::Column::ClubId.eq(tenant.club.id))
        .filter(memberships::Column::UserId.eq(body.user_id))
        .filter(memberships::Column::Status.eq(memberships::STATUS_ACTIVE))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("user is not a member of this club".to_string()))?;
    let target = Users::find_by_id(body.user_id)
        .one(&state.db)
        .await?
        .filter(|u| u.status == users::STATUS_ACTIVE)
        .ok_or_else(|| AppError::NotFound("user is not a member of this club".to_string()))?;
    if target.is_global_admin {
        return Err(AppError::Forbidden(
            "cannot impersonate a global administrator".to_string(),
        ));
    }

    let previous_target = tenant.auth.session.impersonated_user_id;
    // Tenant isolation: a club admin's impersonation is confined to this
    // club; only global admins may follow the target across clubs.
    let club_scope = if tenant.auth.real_user.is_global_admin {
        None
    } else {
        Some(tenant.club.id)
    };
    let txn = state.db.begin().await?;
    if previous_target.is_some() {
        // Switch semantics: one level only, the old impersonation ends here.
        sessions::revoke_session(&txn, tenant.auth.session.id).await?;
    }
    let (token, _) = sessions::create_session(
        &txn,
        &state.config,
        tenant.auth.real_user.id,
        Some(target.id),
        club_scope,
    )
    .await?;
    audit::record(
        &txn,
        tenant.actor(),
        Some(tenant.club.id),
        "impersonation.started",
        "user",
        Some(target.id),
        json!({ "previous_target": previous_target }),
    )
    .await?;
    txn.commit().await?;

    Ok(Json(SessionResponse {
        token,
        user: UserDto::from(&target),
    }))
}

// ---------- audit log ----------

#[derive(Serialize)]
pub struct AuditEntryDto {
    pub id: Uuid,
    pub actor_user_id: Uuid,
    pub impersonated_user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub created_at: sea_orm::prelude::DateTimeWithTimeZone,
}

pub async fn audit_log(
    State(state): State<AppState>,
    tenant: Tenant,
    Query(page): Query<PageParams>,
) -> Result<Json<crate::pagination::Page<AuditEntryDto>>, AppError> {
    tenant.require(Permission::ClubManage)?;
    let (page, per_page) = page.resolve();
    let paginator = AuditLog::find()
        .filter(audit_log::Column::ClubId.eq(tenant.club.id))
        .order_by_desc(audit_log::Column::CreatedAt)
        .paginate(&state.db, per_page);
    let total = paginator.num_items().await? as u64;
    let rows = paginator.fetch_page(page).await?;
    let items = rows
        .into_iter()
        .map(|row| AuditEntryDto {
            id: row.id,
            actor_user_id: row.actor_user_id,
            impersonated_user_id: row.impersonated_user_id,
            action: row.action,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            details: row.details,
            created_at: row.created_at,
        })
        .collect();
    Ok(Json(crate::pagination::Page::new(
        items, total, page, per_page,
    )))
}
