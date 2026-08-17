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

pub async fn create_active_user<C: ConnectionTrait>(
    conn: &C,
    email: &str,
    first_name: &str,
    last_name: &str,
) -> Result<users::Model, AppError> {
    let email = normalize_email(email);
    validate_email(&email)?;
    let first_name = first_name.trim();
    if first_name.is_empty() {
        return Err(AppError::Validation("first name is required".to_string()));
    }
    let created = now();
    Ok(users::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(email),
        first_name: Set(first_name.to_string()),
        last_name: Set(last_name.trim().to_string()),
        status: Set(users::STATUS_ACTIVE.to_string()),
        is_global_admin: Set(false),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(conn)
    .await?)
}

pub async fn activate_if_invited<C: ConnectionTrait>(
    conn: &C,
    user: users::Model,
) -> Result<users::Model, AppError> {
    if user.status != users::STATUS_INVITED {
        return Ok(user);
    }
    let mut active: users::ActiveModel = user.into();
    active.status = Set(users::STATUS_ACTIVE.to_string());
    active.updated_at = Set(now());
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

/// Platform-level enable/disable. `invited` is a lifecycle state owned by the
/// invitation flow and cannot be set through here.
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
