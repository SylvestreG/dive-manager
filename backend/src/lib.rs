pub mod app;
pub mod bootstrap;
pub mod config;
pub mod errors;
pub mod identity;
pub mod mailer;
pub mod maintenance;
pub mod pagination;
pub mod rate_limit;
pub mod tenancy;
#[cfg(test)]
pub mod test_support;

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::mailer::Mailer;
use crate::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
    pub mailer: Arc<Mailer>,
    pub rate: Arc<RateLimiter>,
}

/// Current instant with the timezone representation SeaORM stores.
pub fn now() -> sea_orm::prelude::DateTimeWithTimeZone {
    chrono::Utc::now().fixed_offset()
}

/// Canonical email normalization: trimmed, lowercased. The database CHECK
/// constraint (`email = lower(email)`) backs this up.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn validate_email(email: &str) -> Result<(), errors::AppError> {
    let ok = email.len() <= 254
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
        });
    if ok {
        Ok(())
    } else {
        Err(errors::AppError::Validation(
            "invalid email address".to_string(),
        ))
    }
}
