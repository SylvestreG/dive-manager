use sea_orm::*;
use serde_json::Value;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::audit_log;
use crate::identity::services::sessions::AuthSession;
use crate::now;

/// Who performed an action: always the real user, plus the impersonated
/// target when impersonating.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub user_id: Uuid,
    pub impersonated_user_id: Option<Uuid>,
}

impl From<&AuthSession> for Actor {
    fn from(auth: &AuthSession) -> Self {
        Actor {
            user_id: auth.real_user.id,
            impersonated_user_id: auth.session.impersonated_user_id,
        }
    }
}

pub async fn record<C: ConnectionTrait>(
    conn: &C,
    actor: Actor,
    club_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    details: Value,
) -> Result<(), AppError> {
    audit_log::ActiveModel {
        id: Set(Uuid::now_v7()),
        actor_user_id: Set(actor.user_id),
        impersonated_user_id: Set(actor.impersonated_user_id),
        club_id: Set(club_id),
        action: Set(action.to_string()),
        entity_type: Set(entity_type.to_string()),
        entity_id: Set(entity_id),
        details: Set(details),
        created_at: Set(now()),
    }
    .insert(conn)
    .await?;
    Ok(())
}
