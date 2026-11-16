//! User lifecycle — the ONLY place allowed to write the `users` table.
//! Other modules (tenancy, future business modules) must call these
//! functions, never touch identity entities directly (module-boundary rule,
//! DEVELOPMENT.md).

use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::{prelude::Users, users};
use crate::{normalize_email, now, validate_email};

pub async fn find_by_id<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<users::Model>, AppError> {
    Ok(Users::find_by_id(user_id).one(conn).await?)
}

/// Batched read for list/tree screens (one query per page of ids).
pub async fn find_by_ids<C: ConnectionTrait>(
    conn: &C,
    user_ids: &[Uuid],
) -> Result<Vec<users::Model>, AppError> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(Users::find()
        .filter(users::Column::Id.is_in(user_ids.to_vec()))
        .all(conn)
        .await?)
}

pub async fn list_global_admins<C: ConnectionTrait>(
    conn: &C,
) -> Result<Vec<users::Model>, AppError> {
    Ok(Users::find()
        .filter(users::Column::IsGlobalAdmin.eq(true))
        .order_by_asc(users::Column::Email)
        .all(conn)
        .await?)
}

pub async fn count_active_global_admins<C: ConnectionTrait>(conn: &C) -> Result<u64, AppError> {
    Ok(Users::find()
        .filter(users::Column::IsGlobalAdmin.eq(true))
        .filter(users::Column::Status.ne(users::STATUS_DISABLED))
        .count(conn)
        .await?)
}

pub async fn find_by_email<C: ConnectionTrait>(
    conn: &C,
    email: &str,
) -> Result<Option<users::Model>, AppError> {
    Ok(Users::find()
        .filter(users::Column::Email.eq(normalize_email(email)))
        .one(conn)
        .await?)
}

/// Creates a user on an admin's behalf (manual add, import, approval,
/// bootstrap — ADR-0009: accounts are admin-curated). The account is active
/// but never logged in (`activated_at` NULL) until its first login.
/// `email` is optional; when present it is normalized and must be unique
/// (a duplicate surfaces as 409 through the unique constraint).
pub async fn create_user<C: ConnectionTrait>(
    conn: &C,
    email: Option<&str>,
    first_name: &str,
    last_name: &str,
) -> Result<users::Model, AppError> {
    let email = match email.map(str::trim).filter(|e| !e.is_empty()) {
        Some(raw) => {
            let email = normalize_email(raw);
            validate_email(&email)?;
            Some(email)
        }
        None => None,
    };
    let first_name = first_name.trim();
    let last_name = last_name.trim();
    if first_name.is_empty() || last_name.is_empty() {
        return Err(AppError::Validation(
            "first and last names are required".to_string(),
        ));
    }
    if first_name.len() > 100 || last_name.len() > 100 {
        return Err(AppError::Validation("name too long".to_string()));
    }
    let created = now();
    Ok(users::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(email),
        first_name: Set(first_name.to_string()),
        last_name: Set(last_name.to_string()),
        status: Set(users::STATUS_ACTIVE.to_string()),
        is_global_admin: Set(false),
        activated_at: Set(None),
        email_verified_at: Set(None),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(conn)
    .await?)
}

/// Records a successful login: sets `activated_at` on the first one and,
/// when the login proved the mailbox (magic link), `email_verified_at`.
/// No write when nothing changes.
pub async fn record_login<C: ConnectionTrait>(
    conn: &C,
    user: users::Model,
    email_proven: bool,
) -> Result<users::Model, AppError> {
    let set_activated = user.activated_at.is_none();
    let set_verified = email_proven && user.email.is_some() && user.email_verified_at.is_none();
    if !set_activated && !set_verified {
        return Ok(user);
    }
    let at = now();
    let mut active: users::ActiveModel = user.into();
    if set_activated {
        active.activated_at = Set(Some(at));
    }
    if set_verified {
        active.email_verified_at = Set(Some(at));
    }
    active.updated_at = Set(at);
    Ok(active.update(conn).await?)
}

pub async fn set_global_admin<C: ConnectionTrait>(
    conn: &C,
    user: users::Model,
    is_global_admin: bool,
) -> Result<users::Model, AppError> {
    let mut active: users::ActiveModel = user.into();
    active.is_global_admin = Set(is_global_admin);
    active.updated_at = Set(now());
    Ok(active.update(conn).await?)
}

/// Self-service identity fix: first/last name only (bootstrap admins are
/// created with a name derived from their email, and admin typos happen —
/// the person is the authority on their own name).
pub async fn update_names<C: ConnectionTrait>(
    conn: &C,
    user: users::Model,
    first_name: &str,
    last_name: &str,
) -> Result<users::Model, AppError> {
    let first_name = first_name.trim();
    let last_name = last_name.trim();
    if first_name.is_empty() || last_name.is_empty() {
        return Err(AppError::Validation(
            "first and last names cannot be empty".to_string(),
        ));
    }
    if first_name.len() > 100 || last_name.len() > 100 {
        return Err(AppError::Validation("name too long".to_string()));
    }
    let mut active: users::ActiveModel = user.into();
    active.first_name = Set(first_name.to_string());
    active.last_name = Set(last_name.to_string());
    active.updated_at = Set(now());
    Ok(active.update(conn).await?)
}

/// Platform-level enable/disable.
pub async fn set_status<C: ConnectionTrait>(
    conn: &C,
    user: users::Model,
    status: &str,
) -> Result<users::Model, AppError> {
    if status != users::STATUS_ACTIVE && status != users::STATUS_DISABLED {
        return Err(AppError::Validation("invalid status".to_string()));
    }
    let mut active: users::ActiveModel = user.into();
    active.status = Set(status.to_string());
    active.updated_at = Set(now());
    Ok(active.update(conn).await?)
}
