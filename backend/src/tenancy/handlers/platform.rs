//! Platform endpoints — reserved to real (non-impersonating) global admins.

use axum::extract::{Path, Query, State};
use axum::Json;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::extractors::CurrentUser;
use crate::identity::handlers::UserDto;
use crate::identity::services::sessions::AuthSession;
use crate::identity::services::users as user_service;
use crate::identity::services::{audit, audit::Actor, sessions};
use crate::pagination::PageParams;
use crate::tenancy::entities::{club_roles, clubs, prelude::*};
use crate::tenancy::services::{invitations, seed};
use crate::{normalize_email, now, AppState};

/// Every platform action requires a real, non-impersonating global admin
/// (foundation design §5: sensitive actions refuse impersonating principals).
fn require_real_global_admin(auth: &AuthSession) -> Result<(), AppError> {
    if auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    if !auth.real_user.is_global_admin {
        return Err(AppError::Forbidden(
            "global administrator required".to_string(),
        ));
    }
    Ok(())
}

#[derive(Serialize)]
pub struct ClubDto {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
}

impl From<&clubs::Model> for ClubDto {
    fn from(club: &clubs::Model) -> Self {
        ClubDto {
            id: club.id,
            name: club.name.clone(),
            slug: club.slug.clone(),
            status: club.status.clone(),
        }
    }
}

pub async fn list_clubs(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Query(page): Query<PageParams>,
) -> Result<Json<crate::pagination::Page<ClubDto>>, AppError> {
    require_real_global_admin(&auth)?;
    let (page, per_page) = page.resolve();
    let paginator = Clubs::find()
        .order_by_asc(clubs::Column::Slug)
        .paginate(&state.db, per_page);
    let total = paginator.num_items().await? as u64;
    let rows = paginator.fetch_page(page).await?;
    Ok(Json(crate::pagination::Page::new(
        rows.iter().map(ClubDto::from).collect(),
        total,
        page,
        per_page,
    )))
}

#[derive(Deserialize)]
pub struct CreateClubRequest {
    pub name: String,
    pub slug: String,
}

fn validate_slug(slug: &str) -> Result<String, AppError> {
    let slug = slug.trim().to_lowercase();
    let valid_len = (2..=40).contains(&slug.len());
    let valid_chars = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid_len && valid_chars && !slug.starts_with('-') && !slug.ends_with('-') {
        Ok(slug)
    } else {
        Err(AppError::Validation(
            "slug must be 2-40 characters of a-z, 0-9 or '-'".to_string(),
        ))
    }
}

pub async fn create_club(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<CreateClubRequest>,
) -> Result<Json<ClubDto>, AppError> {
    require_real_global_admin(&auth)?;
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("name is required".to_string()));
    }
    let slug = validate_slug(&body.slug)?;

    let txn = state.db.begin().await?;
    let created = now();
    let club = clubs::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set(name),
        slug: Set(slug),
        status: Set(clubs::STATUS_ACTIVE.to_string()),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(&txn)
    .await?;
    seed::seed_club(&txn, club.id).await?;
    audit::record(
        &txn,
        Actor::from(&auth),
        Some(club.id),
        "club.created",
        "club",
        Some(club.id),
        json!({ "slug": club.slug }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(ClubDto::from(&club)))
}

#[derive(Deserialize)]
pub struct UpdateClubRequest {
    pub name: Option<String>,
    pub status: Option<String>,
}

pub async fn update_club(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(club_id): Path<Uuid>,
    Json(body): Json<UpdateClubRequest>,
) -> Result<Json<ClubDto>, AppError> {
    require_real_global_admin(&auth)?;
    let club = Clubs::find_by_id(club_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("club not found".to_string()))?;

    let mut active: clubs::ActiveModel = club.into();
    if let Some(name) = &body.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("name cannot be empty".to_string()));
        }
        active.name = Set(name.to_string());
    }
    if let Some(status) = &body.status {
        if status != clubs::STATUS_ACTIVE && status != clubs::STATUS_SUSPENDED {
            return Err(AppError::Validation("invalid status".to_string()));
        }
        active.status = Set(status.clone());
    }
    active.updated_at = Set(now());

    // Mutation and audit commit together (review M6).
    let txn = state.db.begin().await?;
    let club = active.update(&txn).await?;
    audit::record(
        &txn,
        Actor::from(&auth),
        Some(club.id),
        "club.updated",
        "club",
        Some(club.id),
        json!({ "name": club.name, "status": club.status }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(ClubDto::from(&club)))
}

#[derive(Deserialize)]
pub struct InviteRequest {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}

/// Invites the club's first (or another) Administrateur.
pub async fn invite_club_admin(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(club_id): Path<Uuid>,
    Json(body): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let club = Clubs::find_by_id(club_id)
        .filter(clubs::Column::Status.eq(clubs::STATUS_ACTIVE))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("club not found".to_string()))?;
    let admin_role = ClubRoles::find()
        .filter(club_roles::Column::ClubId.eq(club.id))
        .filter(club_roles::Column::IsAdmin.eq(true))
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            tracing::error!(club_id = %club.id, "club has no admin role (seed invariant)");
            AppError::Internal
        })?;

    let inviter_name = format!("{} {}", auth.real_user.first_name, auth.real_user.last_name);
    let invitation = invitations::create_invitation(
        &state,
        Actor::from(&auth),
        &inviter_name,
        invitations::NewInvitation {
            email: &body.email,
            first_name: &body.first_name,
            last_name: &body.last_name,
            club: Some(&club),
            role: Some(&admin_role),
        },
    )
    .await?;
    Ok(Json(json!({ "invitation_id": invitation.id })))
}

pub async fn list_global_admins(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<Vec<UserDto>>, AppError> {
    require_real_global_admin(&auth)?;
    let admins = user_service::list_global_admins(&state.db).await?;
    Ok(Json(admins.iter().map(UserDto::from).collect()))
}

/// Grants global-admin to an existing user, or invites a new one.
pub async fn add_global_admin(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let email = normalize_email(&body.email);

    if let Some(user) = user_service::find_by_email(&state.db, &email).await? {
        if user.status == users::STATUS_DISABLED {
            return Err(AppError::Conflict("this account is disabled".to_string()));
        }
        if user.is_global_admin {
            return Err(AppError::Conflict(
                "this user is already a global administrator".to_string(),
            ));
        }
        let txn = state.db.begin().await?;
        let user = user_service::set_global_admin(&txn, user, true).await?;
        audit::record(
            &txn,
            Actor::from(&auth),
            None,
            "global_admin.granted",
            "user",
            Some(user.id),
            json!({ "email": email }),
        )
        .await?;
        txn.commit().await?;
        return Ok(Json(json!({ "granted": true })));
    }

    let inviter_name = format!("{} {}", auth.real_user.first_name, auth.real_user.last_name);
    let invitation = invitations::create_invitation(
        &state,
        Actor::from(&auth),
        &inviter_name,
        invitations::NewInvitation {
            email: &body.email,
            first_name: &body.first_name,
            last_name: &body.last_name,
            club: None,
            role: None,
        },
    )
    .await?;
    Ok(Json(json!({ "invitation_id": invitation.id })))
}

pub async fn remove_global_admin(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    if user_id == auth.real_user.id {
        // Lockout guard: someone else must revoke you.
        return Err(AppError::Validation(
            "cannot revoke your own global-admin status".to_string(),
        ));
    }
    let user = user_service::find_by_id(&state.db, user_id)
        .await?
        .filter(|u| u.is_global_admin)
        .ok_or_else(|| AppError::NotFound("global administrator not found".to_string()))?;

    // The platform must always keep at least one active global admin.
    if user_service::count_active_global_admins(&state.db).await? <= 1 {
        return Err(AppError::Validation(
            "cannot remove the last global administrator".to_string(),
        ));
    }

    let txn = state.db.begin().await?;
    user_service::set_global_admin(&txn, user, false).await?;
    audit::record(
        &txn,
        Actor::from(&auth),
        None,
        "global_admin.revoked",
        "user",
        Some(user_id),
        json!({}),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "revoked": true })))
}

#[derive(Deserialize)]
pub struct SetUserStatusRequest {
    pub status: String,
}

/// Platform-level enable/disable of an account (offboarding, compromise).
/// Disabling revokes every session immediately.
pub async fn set_user_status(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<SetUserStatusRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    if user_id == auth.real_user.id {
        return Err(AppError::Validation(
            "cannot change your own status".to_string(),
        ));
    }
    let user = user_service::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let disabling = body.status == users::STATUS_DISABLED;
    if disabling
        && user.is_global_admin
        && user_service::count_active_global_admins(&state.db).await? <= 1
    {
        return Err(AppError::Validation(
            "cannot disable the last global administrator".to_string(),
        ));
    }

    let txn = state.db.begin().await?;
    user_service::set_status(&txn, user, &body.status).await?;
    if disabling {
        sessions::revoke_all_for_user(&txn, user_id).await?;
    }
    audit::record(
        &txn,
        Actor::from(&auth),
        None,
        "user.status_changed",
        "user",
        Some(user_id),
        json!({ "status": body.status }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "status updated" })))
}
