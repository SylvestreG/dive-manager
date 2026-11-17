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
    // The lookup above matched on this email, so it is present.
    let to = user.email.as_deref().unwrap_or(&email);
    if let Err(error) = state.mailer.send(to, &subject, body).await {
        tracing::error!(?error, "failed to send magic-link email");
    }
    Ok(())
}

/// Consumes a magic link: single use, records the first login
/// (`activated_at`) and proves the email (`email_verified_at`), invalidates
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
    let user = super::users::record_login(&txn, user, true).await?;
    invalidate_pending_links(&txn, user.id).await?;
    let (session_token, _) =
        sessions::create_session(&txn, &state.config, user.id, None, None).await?;
    txn.commit().await?;
    Ok((session_token, user))
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
