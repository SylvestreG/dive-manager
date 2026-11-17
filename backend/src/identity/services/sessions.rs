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
    insert_session(
        conn,
        config,
        user_id,
        impersonated_user_id,
        impersonated_club_id,
        None,
    )
    .await
}

/// A login session (magic link, OIDC exchange) with its device label for
/// the "My devices" page.
pub async fn create_login_session<C: ConnectionTrait>(
    conn: &C,
    config: &Config,
    user_id: Uuid,
    client_label: Option<String>,
) -> Result<(String, auth_sessions::Model), AppError> {
    insert_session(conn, config, user_id, None, None, client_label).await
}

async fn insert_session<C: ConnectionTrait>(
    conn: &C,
    config: &Config,
    user_id: Uuid,
    impersonated_user_id: Option<Uuid>,
    impersonated_club_id: Option<Uuid>,
    client_label: Option<String>,
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
        client_label: Set(client_label),
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

/// The user's live, non-impersonation sessions, most recent use first
/// ("My devices"). Bounded: a person has a handful of devices.
pub async fn list_active_for_user<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<auth_sessions::Model>, AppError> {
    Ok(AuthSessions::find()
        .filter(auth_sessions::Column::UserId.eq(user_id))
        .filter(auth_sessions::Column::ImpersonatedUserId.is_null())
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .filter(auth_sessions::Column::ExpiresAt.gt(now()))
        .order_by_desc(auth_sessions::Column::LastSeenAt)
        .limit(50)
        .all(conn)
        .await?)
}

/// Revokes one of the user's OWN sessions — 404 for anyone else's id.
pub async fn revoke_own<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    let result = AuthSessions::update_many()
        .col_expr(auth_sessions::Column::RevokedAt, Expr::value(Some(now())))
        .filter(auth_sessions::Column::Id.eq(session_id))
        .filter(auth_sessions::Column::UserId.eq(user_id))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .exec(conn)
        .await?;
    if result.rows_affected == 0 {
        return Err(AppError::NotFound("session not found".to_string()));
    }
    Ok(())
}

/// Coarse, privacy-friendly device label from a User-Agent ("Chrome · Mac").
/// Never stores the raw UA nor any IP.
pub fn client_label(user_agent: Option<&str>) -> Option<String> {
    let ua = user_agent?;
    let device = if ua.contains("iPhone") {
        "iPhone"
    } else if ua.contains("iPad") {
        "iPad"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        "Mac"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        return None;
    };
    let browser = if ua.contains("Firefox/") || ua.contains("FxiOS/") {
        Some("Firefox")
    } else if ua.contains("Edg/") || ua.contains("EdgiOS/") {
        Some("Edge")
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") {
        Some("Chrome")
    } else if ua.contains("Safari/") {
        Some("Safari")
    } else {
        None
    };
    Some(match browser {
        Some(browser) => format!("{browser} · {device}"),
        None => device.to_string(),
    })
}

#[cfg(test)]
mod label_tests {
    use super::client_label;

    #[test]
    fn labels_common_user_agents() {
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
        assert_eq!(
            client_label(Some(iphone)).as_deref(),
            Some("Safari · iPhone")
        );
        let chrome_mac = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
        assert_eq!(
            client_label(Some(chrome_mac)).as_deref(),
            Some("Chrome · Mac")
        );
        assert_eq!(client_label(Some("curl/8.0")), None);
        assert_eq!(client_label(None), None);
    }
}
