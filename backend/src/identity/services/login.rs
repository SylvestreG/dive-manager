use std::sync::OnceLock;
use std::time::Duration as StdDuration;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use chrono::Duration;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::{login_magic_links, prelude::*, user_passwords, users};
use crate::identity::services::{emails, sessions, tokens};
use crate::{normalize_email, now, AppState};

const RATE_WINDOW: StdDuration = StdDuration::from_secs(15 * 60);

/// Requests a login magic link. Always succeeds from the caller's point of
/// view (no user enumeration); rate limits and unknown accounts are silent.
pub async fn request_magic_link(
    state: &AppState,
    email: &str,
    client_ip: &str,
) -> Result<(), AppError> {
    let email = normalize_email(email);
    if !state
        .rate
        .check(&format!("ml:email:{email}"), 3, RATE_WINDOW)
        || !state
            .rate
            .check(&format!("ml:ip:{client_ip}"), 20, RATE_WINDOW)
    {
        tracing::warn!(email, client_ip, "magic-link request rate limited");
        return Ok(());
    }

    let Some(user) = Users::find()
        .filter(users::Column::Email.eq(email.clone()))
        .filter(users::Column::Status.is_in([users::STATUS_ACTIVE, users::STATUS_INVITED]))
        .one(&state.db)
        .await?
    else {
        return Ok(());
    };

    // At most one valid link per account (foundation design §5).
    invalidate_pending_links(&state.db, user.id).await?;

    let token = tokens::generate();
    let created = now();
    login_magic_links::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user.id),
        token_hash: Set(token.hash),
        expires_at: Set(created + Duration::hours(state.config.auth.magic_link_hours)),
        consumed_at: Set(None),
        invalidated_at: Set(None),
        created_at: Set(created),
    }
    .insert(&state.db)
    .await?;

    let url = format!("{}/auth/ml/{}", state.config.base_url, token.token);
    let (subject, body) =
        emails::magic_link_email(&user.first_name, &url, state.config.auth.magic_link_hours);
    // A send failure must not turn into an account-existence oracle (the
    // endpoint's response is generic either way); log and stay silent.
    if let Err(error) = state.mailer.send(&user.email, &subject, body).await {
        tracing::error!(?error, "failed to send magic-link email");
    }
    Ok(())
}

/// Consumes a magic link: single use, activates an invited user, invalidates
/// the account's other pending links, opens a session.
pub async fn consume_magic_link(
    state: &AppState,
    token: &str,
) -> Result<(String, users::Model), AppError> {
    let invalid = || AppError::Unauthorized("invalid or expired login link".to_string());
    let hash = tokens::hash(token);
    let link = LoginMagicLinks::find()
        .filter(login_magic_links::Column::TokenHash.eq(hash))
        .filter(login_magic_links::Column::ConsumedAt.is_null())
        .filter(login_magic_links::Column::InvalidatedAt.is_null())
        .filter(login_magic_links::Column::ExpiresAt.gt(now()))
        .one(&state.db)
        .await?
        .ok_or_else(invalid)?;

    let user = Users::find_by_id(link.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(invalid)?;
    if user.status == users::STATUS_DISABLED {
        return Err(invalid());
    }

    let txn = state.db.begin().await?;
    let consumed = LoginMagicLinks::update_many()
        .col_expr(
            login_magic_links::Column::ConsumedAt,
            Expr::value(Some(now())),
        )
        .filter(login_magic_links::Column::Id.eq(link.id))
        .filter(login_magic_links::Column::ConsumedAt.is_null())
        .exec(&txn)
        .await?;
    if consumed.rows_affected != 1 {
        // Lost the race against a concurrent consume: strictly single use.
        return Err(invalid());
    }
    let user = super::users::activate_if_invited(&txn, user).await?;
    invalidate_pending_links(&txn, user.id).await?;
    let (session_token, _) =
        sessions::create_session(&txn, &state.config, user.id, None, None).await?;
    txn.commit().await?;
    Ok((session_token, user))
}

/// Email + password login. Constant-shape behavior on unknown accounts
/// (dummy verification), rate limited per account and per IP.
pub async fn login_password(
    state: &AppState,
    email: &str,
    password: &str,
    client_ip: &str,
) -> Result<(String, users::Model), AppError> {
    let email = normalize_email(email);
    if !state
        .rate
        .check(&format!("pw:email:{email}"), 10, RATE_WINDOW)
        || !state
            .rate
            .check(&format!("pw:ip:{client_ip}"), 30, RATE_WINDOW)
    {
        return Err(AppError::TooManyRequests);
    }
    let invalid = || AppError::Unauthorized("invalid credentials".to_string());

    let found = Users::find()
        .filter(users::Column::Email.eq(email.clone()))
        .filter(users::Column::Status.eq(users::STATUS_ACTIVE))
        .find_also_related(UserPasswords)
        .one(&state.db)
        .await?;

    let (user, stored) = match found {
        Some((user, Some(pw))) => (user, pw.password_hash),
        _ => {
            // Verify against a throwaway hash so response timing does not
            // reveal whether the account or password exists.
            let _ = verify_password("mismatch", dummy_hash()).await;
            return Err(invalid());
        }
    };

    if !verify_password(password, &stored).await {
        return Err(invalid());
    }

    // A successful login should not eat into the account's attempt budget
    // (families sharing a device would lock themselves out).
    state.rate.reset(&format!("pw:email:{email}"));

    let txn = state.db.begin().await?;
    invalidate_pending_links(&txn, user.id).await?;
    let (session_token, _) =
        sessions::create_session(&txn, &state.config, user.id, None, None).await?;
    txn.commit().await?;
    Ok((session_token, user))
}

/// Whether this email logs in with a password. Product rule (2026-08-17):
/// the magic link is only offered to accounts WITHOUT a password. Unknown or
/// disabled emails answer `false` — indistinguishable from a password-less
/// account, so this endpoint only reveals existence for password holders.
pub async fn has_password_login(
    state: &AppState,
    email: &str,
    client_ip: &str,
) -> Result<bool, AppError> {
    let email = normalize_email(email);
    if !state
        .rate
        .check(&format!("lm:email:{email}"), 20, RATE_WINDOW)
        || !state
            .rate
            .check(&format!("lm:ip:{client_ip}"), 60, RATE_WINDOW)
    {
        return Err(AppError::TooManyRequests);
    }
    let found = Users::find()
        .filter(users::Column::Email.eq(email))
        .filter(users::Column::Status.eq(users::STATUS_ACTIVE))
        .find_also_related(UserPasswords)
        .one(&state.db)
        .await?;
    Ok(matches!(found, Some((_, Some(_)))))
}

/// Whether this user has a password credential set (drives the profile UI:
/// "define a password" vs "change the password").
pub async fn user_has_password<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<bool, AppError> {
    Ok(UserPasswords::find_by_id(user_id)
        .one(conn)
        .await?
        .is_some())
}

/// Sets or replaces the user's optional password credential.
pub async fn set_password<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    password: &str,
) -> Result<(), AppError> {
    if password.len() < 8 || password.len() > 128 {
        return Err(AppError::Validation(
            "password must be between 8 and 128 characters".to_string(),
        ));
    }
    let hash = hash_password(password).await?;
    let existing = UserPasswords::find_by_id(user_id).one(conn).await?;
    match existing {
        Some(row) => {
            let mut active: user_passwords::ActiveModel = row.into();
            active.password_hash = Set(hash);
            active.updated_at = Set(now());
            active.update(conn).await?;
        }
        None => {
            user_passwords::ActiveModel {
                user_id: Set(user_id),
                password_hash: Set(hash),
                updated_at: Set(now()),
            }
            .insert(conn)
            .await?;
        }
    }
    Ok(())
}

async fn invalidate_pending_links<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<(), AppError> {
    LoginMagicLinks::update_many()
        .col_expr(
            login_magic_links::Column::InvalidatedAt,
            Expr::value(Some(now())),
        )
        .filter(login_magic_links::Column::UserId.eq(user_id))
        .filter(login_magic_links::Column::ConsumedAt.is_null())
        .filter(login_magic_links::Column::InvalidatedAt.is_null())
        .exec(conn)
        .await?;
    Ok(())
}

/// Argon2 costs ~50-100 ms of pure CPU: it runs on the blocking pool so it
/// never stalls the async workers (review finding H3).
async fn hash_password(password: &str) -> Result<String, AppError> {
    let password = password.to_string();
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "password hashing task failed");
        AppError::Internal
    })?
    .map_err(|e| {
        tracing::error!(error = %e, "password hashing failed");
        AppError::Internal
    })
}

async fn verify_password(password: &str, stored_hash: &str) -> bool {
    let password = password.to_string();
    let stored = stored_hash.to_string();
    tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&stored) else {
            tracing::error!("stored password hash is unparseable");
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(b"dummy-timing-equalizer", &salt)
            .expect("dummy hash")
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn password_hash_verifies_and_rejects() {
        let hash = hash_password("correct horse").await.unwrap();
        assert!(verify_password("correct horse", &hash).await);
        assert!(!verify_password("wrong", &hash).await);
    }
}
