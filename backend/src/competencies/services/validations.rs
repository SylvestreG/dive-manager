//! Skill-validation writes: monotonic progression, append-only history.
//! Every write goes through here — there is no other path (Fosse's PUT
//! bypassed both the grade and the ordering checks).

use chrono::NaiveDate;
use ffessm::Certification;
use sea_orm::*;
use uuid::Uuid;

use crate::competencies::domain::ValidationStage;
use crate::competencies::entities::prelude::*;
use crate::competencies::entities::user_skill_validation_events::{
    self, ACTION_DELETED, ACTION_REVERTED, ACTION_VALIDATED,
};
use crate::competencies::entities::{competency_skills, user_skill_validations};
use crate::errors::AppError;
use crate::now;

pub const MAX_NOTES_CHARS: usize = 2000;

pub async fn find<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    skill_id: Uuid,
) -> Result<Option<user_skill_validations::Model>, AppError> {
    Ok(UserSkillValidations::find()
        .filter(user_skill_validations::Column::UserId.eq(user_id))
        .filter(user_skill_validations::Column::SkillId.eq(skill_id))
        .one(conn)
        .await?)
}

/// Batched read: the validations of one user over a set of skills (a level).
pub async fn list_for_user<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    skill_ids: &[Uuid],
) -> Result<Vec<user_skill_validations::Model>, AppError> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(UserSkillValidations::find()
        .filter(user_skill_validations::Column::UserId.eq(user_id))
        .filter(user_skill_validations::Column::SkillId.is_in(skill_ids.to_vec()))
        .all(conn)
        .await?)
}

/// Batched read for the students list: all validations of a page of users
/// over a level's skills.
pub async fn list_for_users<C: ConnectionTrait>(
    conn: &C,
    user_ids: &[Uuid],
    skill_ids: &[Uuid],
) -> Result<Vec<user_skill_validations::Model>, AppError> {
    if user_ids.is_empty() || skill_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(UserSkillValidations::find()
        .filter(user_skill_validations::Column::UserId.is_in(user_ids.to_vec()))
        .filter(user_skill_validations::Column::SkillId.is_in(skill_ids.to_vec()))
        .all(conn)
        .await?)
}

pub struct WriteRequest<'a> {
    pub user_id: Uuid,
    pub skill: &'a competency_skills::Model,
    pub certification: Certification,
    pub stage: ValidationStage,
    pub validated_at: NaiveDate,
    /// The RECORDED validator (current state, printed on the PDF) — equals
    /// the actor except when a global admin attributes the validation to a
    /// named instructor (product decision 2026-08-20).
    pub validated_by_user_id: Uuid,
    /// The acting principal (history events always record who acted).
    pub performed_by_user_id: Uuid,
    /// Club context of the write, if any (None for platform-side rollbacks).
    pub performed_in_club_id: Option<Uuid>,
    pub notes: Option<String>,
    /// True only for a REAL global admin: allows moving the stage backwards.
    pub allow_regression: bool,
}

/// Upsert of the current state + one immutable event. Callers wrap this in a
/// transaction (state + event + audit must land together).
pub async fn write<C: ConnectionTrait>(
    conn: &C,
    request: WriteRequest<'_>,
) -> Result<user_skill_validations::Model, AppError> {
    if request.validated_at > chrono::Utc::now().date_naive() {
        return Err(AppError::Validation(
            "validation date cannot be in the future".to_string(),
        ));
    }
    let notes = request
        .notes
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    if notes
        .as_ref()
        .is_some_and(|n| n.chars().count() > MAX_NOTES_CHARS)
    {
        return Err(AppError::Validation("notes are too long".to_string()));
    }

    let existing = find(conn, request.user_id, request.skill.id).await?;
    let previous_stage = existing
        .as_ref()
        .and_then(|row| ValidationStage::parse(&row.stage));
    let regression = previous_stage.is_some_and(|previous| request.stage < previous);
    if regression && !request.allow_regression {
        return Err(AppError::Conflict(
            "validation stages only move forward".to_string(),
        ));
    }

    let current = now();
    let row = match existing {
        None => {
            let inserted = user_skill_validations::ActiveModel {
                id: Set(Uuid::now_v7()),
                user_id: Set(request.user_id),
                skill_id: Set(request.skill.id),
                stage: Set(request.stage.code().to_string()),
                validated_at: Set(request.validated_at),
                validated_by_user_id: Set(Some(request.validated_by_user_id)),
                validated_in_club_id: Set(request.performed_in_club_id),
                notes: Set(notes.clone()),
                created_at: Set(current),
                updated_at: Set(current),
            }
            .insert(conn)
            .await;
            // Two concurrent FIRST validations of the same (user, skill):
            // the unique index protects integrity; give the loser a domain
            // message instead of the generic "resource already exists".
            match inserted.map_err(AppError::from) {
                Ok(row) => row,
                Err(AppError::Conflict(_)) => {
                    return Err(AppError::Conflict(
                        "this skill was just validated by someone else, reload and retry"
                            .to_string(),
                    ));
                }
                Err(other) => return Err(other),
            }
        }
        Some(row) => {
            let mut active: user_skill_validations::ActiveModel = row.into();
            active.stage = Set(request.stage.code().to_string());
            active.validated_at = Set(request.validated_at);
            active.validated_by_user_id = Set(Some(request.validated_by_user_id));
            active.validated_in_club_id = Set(request.performed_in_club_id);
            active.notes = Set(notes.clone());
            active.updated_at = Set(current);
            active.update(conn).await?
        }
    };

    append_event(
        conn,
        Event {
            user_id: request.user_id,
            skill: Some(request.skill),
            skill_name: &request.skill.name,
            certification: request.certification,
            action: if regression {
                ACTION_REVERTED
            } else {
                ACTION_VALIDATED
            },
            stage: Some(request.stage),
            previous_stage,
            validated_at: Some(request.validated_at),
            performed_by_user_id: request.performed_by_user_id,
            performed_in_club_id: request.performed_in_club_id,
            notes,
        },
    )
    .await?;
    Ok(row)
}

/// Removes the current state; the `deleted` event keeps the trace forever.
pub async fn delete<C: ConnectionTrait>(
    conn: &C,
    row: user_skill_validations::Model,
    skill: &competency_skills::Model,
    certification: Certification,
    performed_by_user_id: Uuid,
    performed_in_club_id: Option<Uuid>,
) -> Result<(), AppError> {
    let previous_stage = ValidationStage::parse(&row.stage);
    let user_id = row.user_id;
    let active: user_skill_validations::ActiveModel = row.into();
    active.delete(conn).await?;
    append_event(
        conn,
        Event {
            user_id,
            skill: Some(skill),
            skill_name: &skill.name,
            certification,
            action: ACTION_DELETED,
            stage: None,
            previous_stage,
            validated_at: None,
            performed_by_user_id,
            performed_in_club_id,
            notes: None,
        },
    )
    .await?;
    Ok(())
}

struct Event<'a> {
    user_id: Uuid,
    skill: Option<&'a competency_skills::Model>,
    skill_name: &'a str,
    certification: Certification,
    action: &'static str,
    stage: Option<ValidationStage>,
    previous_stage: Option<ValidationStage>,
    validated_at: Option<NaiveDate>,
    performed_by_user_id: Uuid,
    performed_in_club_id: Option<Uuid>,
    notes: Option<String>,
}

async fn append_event<C: ConnectionTrait>(conn: &C, event: Event<'_>) -> Result<(), AppError> {
    user_skill_validation_events::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(event.user_id),
        skill_id: Set(event.skill.map(|s| s.id)),
        skill_name: Set(event.skill_name.to_string()),
        certification: Set(event.certification.code().to_string()),
        action: Set(event.action.to_string()),
        stage: Set(event.stage.map(|s| s.code().to_string())),
        previous_stage: Set(event.previous_stage.map(|s| s.code().to_string())),
        validated_at: Set(event.validated_at),
        performed_by_user_id: Set(Some(event.performed_by_user_id)),
        performed_in_club_id: Set(event.performed_in_club_id),
        notes: Set(event.notes),
        created_at: Set(now()),
    }
    .insert(conn)
    .await?;
    Ok(())
}

/// Paginated history, newest first, optionally filtered on one user.
pub async fn events_page(
    db: &DatabaseConnection,
    user_id: Option<Uuid>,
    page: u64,
    per_page: u64,
) -> Result<(Vec<user_skill_validation_events::Model>, u64), AppError> {
    let mut query = UserSkillValidationEvents::find();
    if let Some(user_id) = user_id {
        query = query.filter(user_skill_validation_events::Column::UserId.eq(user_id));
    }
    let paginator = query
        .order_by_desc(user_skill_validation_events::Column::CreatedAt)
        .paginate(db, per_page);
    let total = paginator.num_items().await? as u64;
    let rows = paginator.fetch_page(page).await?;
    Ok((rows, total))
}

/// Cascade impact of deleting a subtree: how many member validations would
/// be destroyed (shown by the admin UI before confirming).
pub async fn count_for_skills<C: ConnectionTrait>(
    conn: &C,
    skill_ids: &[Uuid],
) -> Result<u64, AppError> {
    if skill_ids.is_empty() {
        return Ok(0);
    }
    Ok(UserSkillValidations::find()
        .filter(user_skill_validations::Column::SkillId.is_in(skill_ids.to_vec()))
        .count(conn)
        .await?)
}
