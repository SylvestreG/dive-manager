//! Dated transitional statuses that are NOT certifications (currently only
//! the stagiaire pédagogique MF1 — E2 while the livret is valid, never DP).

use chrono::NaiveDate;
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::now;
use crate::users::entities::prelude::*;
use crate::users::entities::user_training_statuses::{self, STATUS_STAGIAIRE_MF1};

pub async fn list<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<user_training_statuses::Model>, AppError> {
    Ok(UserTrainingStatuses::find()
        .filter(user_training_statuses::Column::UserId.eq(user_id))
        .order_by_asc(user_training_statuses::Column::Status)
        .all(conn)
        .await?)
}

/// Batched variant for list screens.
pub async fn list_for_users<C: ConnectionTrait>(
    conn: &C,
    user_ids: &[Uuid],
) -> Result<Vec<user_training_statuses::Model>, AppError> {
    Ok(UserTrainingStatuses::find()
        .filter(user_training_statuses::Column::UserId.is_in(user_ids.to_vec()))
        .all(conn)
        .await?)
}

/// Sets (valid_until = Some) or removes (None) the stagiaire MF1 status.
pub async fn set_stagiaire_mf1<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    valid_until: Option<NaiveDate>,
) -> Result<(), AppError> {
    let existing = UserTrainingStatuses::find()
        .filter(user_training_statuses::Column::UserId.eq(user_id))
        .filter(user_training_statuses::Column::Status.eq(STATUS_STAGIAIRE_MF1))
        .one(conn)
        .await?;
    match (existing, valid_until) {
        (Some(row), Some(date)) => {
            let mut active: user_training_statuses::ActiveModel = row.into();
            active.valid_until = Set(date);
            active.update(conn).await?;
        }
        (Some(row), None) => {
            let active: user_training_statuses::ActiveModel = row.into();
            active.delete(conn).await?;
        }
        (None, Some(date)) => {
            user_training_statuses::ActiveModel {
                id: Set(Uuid::now_v7()),
                user_id: Set(user_id),
                status: Set(STATUS_STAGIAIRE_MF1.to_string()),
                valid_until: Set(date),
                created_at: Set(now()),
            }
            .insert(conn)
            .await?;
        }
        (None, None) => {}
    }
    Ok(())
}
