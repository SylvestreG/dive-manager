//! User certifications with provenance (docs/design/users-module.md §2).

use chrono::NaiveDate;
use ffessm::Certification;
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::now;
use crate::users::entities::prelude::*;
use crate::users::entities::user_certifications::{self, provenance_rank};
use crate::users::entities::user_profiles;

pub async fn list<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<user_certifications::Model>, AppError> {
    Ok(UserCertifications::find()
        .filter(user_certifications::Column::UserId.eq(user_id))
        .order_by_asc(user_certifications::Column::Certification)
        .all(conn)
        .await?)
}

/// Batched variant for list screens (one query for a page of members).
pub async fn list_for_users<C: ConnectionTrait>(
    conn: &C,
    user_ids: &[Uuid],
) -> Result<Vec<user_certifications::Model>, AppError> {
    Ok(UserCertifications::find()
        .filter(user_certifications::Column::UserId.is_in(user_ids.to_vec()))
        .all(conn)
        .await?)
}

pub struct NewCertification {
    pub certification: String,
    pub obtained_on: Option<NaiveDate>,
    pub provenance: &'static str,
    pub recorded_by_user_id: Option<Uuid>,
    pub recorded_in_club_id: Option<Uuid>,
}

/// Inserts or upgrades a certification, then ends a matching preparation.
/// One row per (user, certification); a write may only keep-or-raise the
/// provenance rank, else 409.
pub async fn upsert<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    new: NewCertification,
) -> Result<user_certifications::Model, AppError> {
    let code = new.certification.clone();
    let row = upsert_row(conn, user_id, new).await?;
    clear_preparing_among(conn, user_id, std::slice::from_ref(&code)).await?;
    Ok(row)
}

/// Core write WITHOUT the preparing purge — batch callers (licence import)
/// purge once at the end instead of re-reading the profile per row.
pub(crate) async fn upsert_row<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    new: NewCertification,
) -> Result<user_certifications::Model, AppError> {
    Certification::parse(&new.certification).ok_or_else(|| {
        AppError::Validation(format!("unknown certification: {}", new.certification))
    })?;
    if let Some(date) = new.obtained_on {
        use chrono::Datelike;
        if date > chrono::Utc::now().date_naive() || date.year_ce().1 < 1900 {
            return Err(AppError::Validation("invalid obtained_on date".to_string()));
        }
    }

    let existing = UserCertifications::find()
        .filter(user_certifications::Column::UserId.eq(user_id))
        .filter(user_certifications::Column::Certification.eq(new.certification.clone()))
        .one(conn)
        .await?;
    let current = now();
    let row = match existing {
        None => {
            user_certifications::ActiveModel {
                id: Set(Uuid::now_v7()),
                user_id: Set(user_id),
                certification: Set(new.certification),
                obtained_on: Set(new.obtained_on),
                provenance: Set(new.provenance.to_string()),
                recorded_by_user_id: Set(new.recorded_by_user_id),
                recorded_in_club_id: Set(new.recorded_in_club_id),
                created_at: Set(current),
                updated_at: Set(current),
            }
            .insert(conn)
            .await?
        }
        Some(row) => {
            if provenance_rank(new.provenance) < provenance_rank(&row.provenance) {
                return Err(AppError::Conflict(
                    "certification already recorded with a stronger provenance".to_string(),
                ));
            }
            let mut active: user_certifications::ActiveModel = row.into();
            active.provenance = Set(new.provenance.to_string());
            if new.obtained_on.is_some() {
                active.obtained_on = Set(new.obtained_on);
            }
            active.recorded_by_user_id = Set(new.recorded_by_user_id);
            active.recorded_in_club_id = Set(new.recorded_in_club_id);
            active.updated_at = Set(current);
            active.update(conn).await?
        }
    };
    Ok(row)
}

/// Data hygiene: obtaining a certification ends its preparation. One profile
/// read whatever the batch size.
pub(crate) async fn clear_preparing_among<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    obtained_codes: &[String],
) -> Result<(), AppError> {
    let profile = UserProfiles::find_by_id(user_id).one(conn).await?;
    if let Some(profile) = profile {
        let preparing_obtained = profile
            .preparing_certification
            .as_deref()
            .is_some_and(|preparing| obtained_codes.iter().any(|c| c == preparing));
        if preparing_obtained {
            let mut active: user_profiles::ActiveModel = profile.into();
            active.preparing_certification = Set(None);
            active.updated_at = Set(now());
            active.update(conn).await?;
        }
    }
    Ok(())
}

/// Loads a certification row belonging to `user_id` (404 otherwise).
pub async fn find_of_user<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    certification_id: Uuid,
) -> Result<user_certifications::Model, AppError> {
    UserCertifications::find_by_id(certification_id)
        .filter(user_certifications::Column::UserId.eq(user_id))
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("certification not found".to_string()))
}

pub async fn delete<C: ConnectionTrait>(
    conn: &C,
    row: user_certifications::Model,
) -> Result<(), AppError> {
    let active: user_certifications::ActiveModel = row.into();
    active.delete(conn).await?;
    Ok(())
}
