use std::time::Duration as StdDuration;

use chrono::Duration;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::{login_magic_links, prelude::*, users};
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
        .filter(users::Column::Status.eq(users::STATUS_ACTIVE))
        .one(&state.db)
        .await?
    else {
        return Ok(());
    };

    // At most one valid link per account (foundation design §5).
    invalidate_pending_links(&state.db, user.id, login_magic_links::PURPOSE_LOGIN).await?;

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
        purpose: Set(login_magic_links::PURPOSE_LOGIN.to_string()),
        payload: Set(serde_json::json!({})),
    }
    .insert(&state.db)
    .await?;

    let url = format!("{}/auth/ml/{}", state.config.base_url, token.token);
    let (subject, body) =
        emails::magic_link_email(&user.first_name, &url, state.config.auth.magic_link_hours);
    // A send failure must not turn into an account-existence oracle (the
    // endpoint's response is generic either way); log and stay silent.
    // The lookup above matched on this email, so it is present.
    let to = user.email.as_deref().unwrap_or(&email);
    if let Err(error) = state.mailer.send(to, &subject, body).await {
        tracing::error!(?error, "failed to send magic-link email");
    }
    Ok(())
}

/// Consumes a magic link (single use) and opens a session — clicking it
/// proves the mailbox. Purposes (ADR-0009):
/// - `login`: plain login;
/// - `confirm_link`: also links the OIDC identity carried in the payload;
/// - `confirm_email`: also switches the account email to the new address.
///
/// Records the first login (`activated_at`) and `email_verified_at`, and
/// invalidates the account's other pending links of the same purpose.
pub async fn consume_magic_link(
    state: &AppState,
    token: &str,
    client_label: Option<String>,
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
    let user = match link.purpose.as_str() {
        login_magic_links::PURPOSE_LOGIN => user,
        login_magic_links::PURPOSE_CONFIRM_LINK => {
            super::oidc::apply_confirmed_link(&txn, user.id, &link.payload).await?;
            user
        }
        login_magic_links::PURPOSE_CONFIRM_EMAIL => {
            let new_email = link.payload["new_email"].as_str().ok_or_else(invalid)?;
            let old_email = user.email.clone();
            let user = super::users::set_email(&txn, user, new_email).await?;
            // The old mailbox may be lost or compromised (that is often WHY
            // the address changes): every pending link sent there dies.
            invalidate_all_pending_links(&txn, user.id).await?;
            super::audit::record(
                &txn,
                super::audit::Actor {
                    user_id: user.id,
                    impersonated_user_id: None,
                },
                None,
                "email.changed",
                "user",
                Some(user.id),
                serde_json::json!({ "from": old_email, "to": user.email }),
            )
            .await?;
            user
        }
        _ => return Err(invalid()),
    };
    let user = super::users::record_login(&txn, user, true).await?;
    invalidate_pending_links(&txn, user.id, &link.purpose).await?;
    let (session_token, _) =
        sessions::create_login_session(&txn, &state.config, user.id, client_label).await?;
    txn.commit().await?;
    Ok((session_token, user))
}

async fn invalidate_all_pending_links<C: ConnectionTrait>(
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

async fn invalidate_pending_links<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    purpose: &str,
) -> Result<(), AppError> {
    LoginMagicLinks::update_many()
        .col_expr(
            login_magic_links::Column::InvalidatedAt,
            Expr::value(Some(now())),
        )
        .filter(login_magic_links::Column::UserId.eq(user_id))
        .filter(login_magic_links::Column::Purpose.eq(purpose))
        .filter(login_magic_links::Column::ConsumedAt.is_null())
        .filter(login_magic_links::Column::InvalidatedAt.is_null())
        .exec(conn)
        .await?;
    Ok(())
}

/// R12: an account email change takes effect only when the NEW address
/// clicks its confirmation link. An address used by another account gets
/// the same answer but no email (no account-existence oracle); the unique
/// constraint backstops the race at confirmation time.
pub async fn request_email_change(
    state: &AppState,
    user: &users::Model,
    new_email: &str,
) -> Result<(), AppError> {
    let new_email = normalize_email(new_email);
    crate::validate_email(&new_email)?;
    if user.email.as_deref() == Some(new_email.as_str()) {
        return Err(AppError::Validation(
            "this is already your email".to_string(),
        ));
    }
    if !state
        .rate
        .check(&format!("email-change:{}", user.id), 3, RATE_WINDOW)
    {
        return Err(AppError::TooManyRequests);
    }
    if super::users::find_by_email(&state.db, &new_email)
        .await?
        .is_some()
    {
        tracing::info!(user_id = %user.id, "email change to an address in use: silently ignored");
        return Ok(());
    }
    invalidate_pending_links(&state.db, user.id, login_magic_links::PURPOSE_CONFIRM_EMAIL).await?;
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
        purpose: Set(login_magic_links::PURPOSE_CONFIRM_EMAIL.to_string()),
        payload: Set(serde_json::json!({ "new_email": new_email })),
    }
    .insert(&state.db)
    .await?;
    let url = format!("{}/auth/ml/{}", state.config.base_url, token.token);
    let (subject, body) =
        emails::email_change_email(&user.first_name, &url, state.config.auth.magic_link_hours);
    state.mailer.send(&new_email, &subject, body).await?;
    Ok(())
}
