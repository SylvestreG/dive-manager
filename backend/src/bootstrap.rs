use sea_orm::*;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::identity::entities::{prelude::Users, users};
use crate::{normalize_email, now, validate_email};

/// Bootstraps global admins from configuration — only while the database has
/// none (ADR-0004 §1). Afterwards the database is the sole source of truth
/// and this list is ignored entirely.
pub async fn bootstrap_global_admins(
    db: &DatabaseConnection,
    config: &Config,
) -> Result<(), AppError> {
    let existing = Users::find()
        .filter(users::Column::IsGlobalAdmin.eq(true))
        .count(db)
        .await?;
    if existing > 0 {
        return Ok(());
    }
    if config.auth.bootstrap_admin_emails.is_empty() {
        tracing::warn!("no global admin in database and no bootstrap_admin_emails configured");
        return Ok(());
    }

    for raw_email in &config.auth.bootstrap_admin_emails {
        let email = normalize_email(raw_email);
        if let Err(e) = validate_email(&email) {
            tracing::error!(email, error = %e, "skipping invalid bootstrap admin email");
            continue;
        }
        match Users::find()
            .filter(users::Column::Email.eq(Some(email.clone())))
            .one(db)
            .await?
        {
            Some(user) => {
                let mut active: users::ActiveModel = user.into();
                active.is_global_admin = Set(true);
                active.updated_at = Set(now());
                active.update(db).await?;
                tracing::info!(email, "bootstrap: promoted existing user to global admin");
            }
            None => {
                let first_name = email.split('@').next().unwrap_or("admin").to_string();
                let created = now();
                users::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    email: Set(Some(email.clone())),
                    first_name: Set(first_name),
                    last_name: Set(String::new()),
                    status: Set(users::STATUS_ACTIVE.to_string()),
                    is_global_admin: Set(true),
                    activated_at: Set(None),
                    email_verified_at: Set(None),
                    created_at: Set(created),
                    updated_at: Set(created),
                }
                .insert(db)
                .await?;
                tracing::info!(
                    email,
                    "bootstrap: created global admin (first login via magic link)"
                );
            }
        }
    }
    Ok(())
}
