use chrono::Duration;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::identity::entities::{auth_sessions, prelude::*, users};
use crate::identity::services::tokens;
use crate::now;

/// Result of authenticating a bearer token. `effective_user` is the
/// impersonated user when impersonating, else the real user.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub session: auth_sessions::Model,
    pub real_user: users::Model,
    pub effective_user: users::Model,
}

impl AuthSession {
    pub fn is_impersonating(&self) -> bool {
        self.session.impersonated_user_id.is_some()
    }
}

/// Creates a revocable server-side session and returns its opaque token
/// (stored hashed — ADR-0004 §4). `impersonated_club_id` confines a
/// club-admin impersonation to that club; pass None for global admins.
pub async fn create_session<C: ConnectionTrait>(
    conn: &C,
    config: &Config,
    user_id: Uuid,
    impersonated_user_id: Option<Uuid>,
    impersonated_club_id: Option<Uuid>,
) -> Result<(String, auth_sessions::Model), AppError> {
    let token = tokens::generate();
    let created = now();
    let session = auth_sessions::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        token_hash: Set(token.hash),
        impersonated_user_id: Set(impersonated_user_id),
        impersonated_club_id: Set(impersonated_club_id),
        created_at: Set(created),
        last_seen_at: Set(created),
        expires_at: Set(created + Duration::days(config.auth.session_idle_days)),
        revoked_at: Set(None),
    }
    .insert(conn)
    .await?;
    Ok((token.token, session))
}

/// Validates a bearer token: hash lookup, revocation, sliding idle expiry and
/// absolute cap, user status. Authorization state is always read from the
/// database, so role changes take effect on the next request.
pub async fn authenticate(
    db: &DatabaseConnection,
    config: &Config,
    token: &str,
) -> Result<AuthSession, AppError> {
    let invalid = || AppError::Unauthorized("invalid or expired session".to_string());
    let hash = tokens::hash(token);
    let session = AuthSessions::find()
        .filter(auth_sessions::Column::TokenHash.eq(hash))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(invalid)?;

    let current = now();
    let hard_cap = session.created_at + Duration::days(config.auth.session_max_days);
    if current >= session.expires_at || current >= hard_cap {
        return Err(invalid());
    }

    // Sliding expiry, throttled to one write per minute per session.
    let mut session = session;
    if (current - session.last_seen_at) > Duration::seconds(60) {
        let new_expiry = (current + Duration::days(config.auth.session_idle_days)).min(hard_cap);
        let mut touch: auth_sessions::ActiveModel = session.clone().into();
        touch.last_seen_at = Set(current);
        touch.expires_at = Set(new_expiry);
        session = touch.update(db).await?;
    }

    let real_user = Users::find_by_id(session.user_id)
        .one(db)
        .await?
        .ok_or_else(invalid)?;
    if real_user.status != users::STATUS_ACTIVE {
        return Err(invalid());
    }

    let effective_user = match session.impersonated_user_id {
        None => real_user.clone(),
        Some(target_id) => {
            let target = Users::find_by_id(target_id)
                .one(db)
                .await?
                .ok_or_else(invalid)?;
            if target.status != users::STATUS_ACTIVE {
                return Err(invalid());
            }
            target
        }
    };

    Ok(AuthSession {
        session,
        real_user,
        effective_user,
    })
}

pub async fn revoke_session<C: ConnectionTrait>(
    conn: &C,
    session_id: Uuid,
) -> Result<(), AppError> {
    AuthSessions::update_many()
        .col_expr(auth_sessions::Column::RevokedAt, Expr::value(Some(now())))
        .filter(auth_sessions::Column::Id.eq(session_id))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .exec(conn)
        .await?;
    Ok(())
}

pub async fn revoke_all_for_user<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<(), AppError> {
    AuthSessions::update_many()
        .col_expr(auth_sessions::Column::RevokedAt, Expr::value(Some(now())))
        .filter(auth_sessions::Column::UserId.eq(user_id))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .exec(conn)
        .await?;
    Ok(())
}
