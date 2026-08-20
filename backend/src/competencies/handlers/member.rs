//! Club-scoped competency routes. The tree and the validations are
//! user-global; what the club context adds is VISIBILITY (target must be an
//! active member — 404 otherwise) and the validator gate (certified
//! instructor of the required grade, active member of this club).

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::response::Response;
use axum::Json;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::competencies::domain::{required_validator_grade, ValidationStage};
use crate::competencies::handlers::{
    competency_view, documents, teaching_grade, CompetencyViewDto, LevelRulesDto,
};
use crate::competencies::services::progress::Progress;
use crate::competencies::services::{tree, validations};
use crate::errors::AppError;
use crate::identity::handlers::UserDto;
use crate::identity::services::audit;
use crate::identity::services::users as user_service;
use crate::pagination::{Page, PageParams};
use crate::tenancy::context::Tenant;
use crate::tenancy::services::memberships;
use crate::users::access::{ensure_member, require_members_view};
use crate::users::services::profiles;
use crate::AppState;

#[derive(Deserialize)]
pub struct LevelQuery {
    pub certification: String,
}

pub async fn get_member_competencies(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
    Query(query): Query<LevelQuery>,
) -> Result<Json<CompetencyViewDto>, AppError> {
    require_members_view(&state, &tenant).await?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    let certification = tree::parse_certification(&query.certification)?;
    Ok(Json(
        competency_view(&state.db, user.id, certification).await?,
    ))
}

pub async fn get_member_document(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
    Query(query): Query<LevelQuery>,
) -> Result<Response, AppError> {
    require_members_view(&state, &tenant).await?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    let certification = tree::parse_certification(&query.certification)?;
    documents::generate_for_user(&state, &user, certification).await
}

#[derive(Deserialize)]
pub struct ValidationRequest {
    pub stage: String,
    /// Defaults to today; backdating is allowed, future dates are not.
    pub validated_at: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
    /// Real global admins only (product decision 2026-08-20): attribute the
    /// validation to a named instructor ("valider au nom d'un encadrant").
    /// The designated instructor must be a legitimate validator here.
    pub validated_by_user_id: Option<Uuid>,
}

/// THE single write path for skill validations (Fosse's PUT bypassed every
/// check). Forward-only; a REAL global admin may regress, and may attribute
/// the validation to a named instructor.
pub async fn put_skill_validation(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id, skill_id)): Path<(String, Uuid, Uuid)>,
    Json(body): Json<ValidationRequest>,
) -> Result<Json<CompetencyViewDto>, AppError> {
    // Rule 9: competency writes are never performed while impersonating —
    // Fosse silently attributed them to the impersonated instructor.
    if tenant.auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    let target = ensure_member(&state, &tenant, user_id).await?;
    let skill = tree::find_skill(&state.db, skill_id).await?;
    let certification = tree::certification_of_skill(&state.db, &skill).await?;
    let stage = ValidationStage::parse(&body.stage)
        .ok_or_else(|| AppError::Validation(format!("unknown stage: {}", body.stage)))?;
    let required = required_validator_grade(certification);

    // Real global admins operate the platform: no grade gate, and they alone
    // may move a stage backwards (product decision 2026-08-19).
    let actor_user_id = tenant.auth.real_user.id;
    let is_real_global_admin = tenant.auth.real_user.is_global_admin;
    if !is_real_global_admin {
        if tenant.membership.is_none() {
            return Err(AppError::Forbidden(
                "validating requires being a member of this club".to_string(),
            ));
        }
        let held_grade = teaching_grade(&state.db, actor_user_id).await?;
        if !held_grade.is_some_and(|grade| grade >= required) {
            return Err(AppError::Forbidden(format!(
                "validating this level requires a certified {required} instructor"
            )));
        }
    }

    // Attribution: the recorded validator is the actor, unless a real global
    // admin designates an instructor — who must then pass the exact gates a
    // direct validator would (active co-member, sufficient derived grade).
    let mut validated_by_user_id = actor_user_id;
    if let Some(designated) = body.validated_by_user_id {
        if designated != actor_user_id {
            if !is_real_global_admin {
                return Err(AppError::Forbidden(
                    "attributing a validation is reserved for global administrators".to_string(),
                ));
            }
            if memberships::find_active(&state.db, tenant.club.id, designated)
                .await?
                .is_none()
            {
                return Err(AppError::Validation(
                    "the designated validator is not an active member of this club".to_string(),
                ));
            }
            let designated_grade = teaching_grade(&state.db, designated).await?;
            if !designated_grade.is_some_and(|grade| grade >= required) {
                return Err(AppError::Validation(format!(
                    "the designated validator must be a certified {required} instructor"
                )));
            }
            validated_by_user_id = designated;
        }
    }
    // Rule 7 targets the RECORDED validator (an admin may thus have their own
    // skills validated by attributing them to a real instructor).
    if validated_by_user_id == target.id {
        return Err(AppError::Validation(
            "you cannot validate your own skills".to_string(),
        ));
    }

    let validated_at = body
        .validated_at
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let actor = tenant.actor();
    let txn = state.db.begin().await?;
    let row = validations::write(
        &txn,
        validations::WriteRequest {
            user_id: target.id,
            skill: &skill,
            certification,
            stage,
            validated_at,
            validated_by_user_id,
            performed_by_user_id: actor_user_id,
            performed_in_club_id: Some(tenant.club.id),
            notes: body.notes,
            allow_regression: is_real_global_admin,
        },
    )
    .await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "competencies.validation.written",
        "user_skill_validation",
        Some(row.id),
        json!({
            "user_id": target.id,
            "skill_id": skill.id,
            "skill_name": skill.name,
            "stage": stage.code(),
            "on_behalf_of": (validated_by_user_id != actor_user_id)
                .then_some(validated_by_user_id),
        }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(
        competency_view(&state.db, target.id, certification).await?,
    ))
}

/// Rollback: real global admin only; the `deleted` event keeps the trace.
pub async fn delete_skill_validation(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id, skill_id)): Path<(String, Uuid, Uuid)>,
) -> Result<Json<CompetencyViewDto>, AppError> {
    if tenant.auth.is_impersonating() || !tenant.auth.real_user.is_global_admin {
        return Err(AppError::Forbidden(
            "global administrator required".to_string(),
        ));
    }
    let target = ensure_member(&state, &tenant, user_id).await?;
    let skill = tree::find_skill(&state.db, skill_id).await?;
    let certification = tree::certification_of_skill(&state.db, &skill).await?;
    let row = validations::find(&state.db, target.id, skill.id)
        .await?
        .ok_or_else(|| AppError::NotFound("validation not found".to_string()))?;

    let actor = tenant.actor();
    let row_id = row.id;
    let txn = state.db.begin().await?;
    validations::delete(
        &txn,
        row,
        &skill,
        certification,
        tenant.auth.real_user.id,
        Some(tenant.club.id),
    )
    .await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "competencies.validation.deleted",
        "user_skill_validation",
        Some(row_id),
        json!({ "user_id": target.id, "skill_id": skill.id, "skill_name": skill.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(
        competency_view(&state.db, target.id, certification).await?,
    ))
}

// ---- students list ----

/// Explicit fields — `#[serde(flatten)]` breaks integer parsing in axum's
/// `Query` (serde_urlencoded buffers values as strings): `?page=2` was a 400.
#[derive(Deserialize)]
pub struct StudentsQuery {
    pub certification: String,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Serialize)]
pub struct StudentDto {
    pub user: UserDto,
    pub progress: Progress,
}

#[derive(Serialize)]
pub struct StudentsResponse {
    pub level: LevelRulesDto,
    #[serde(flatten)]
    pub page: Page<StudentDto>,
}

/// Active members of the club preparing this level, with their progress —
/// a paginated read built from batched queries (never per-student).
pub async fn list_students(
    State(state): State<AppState>,
    tenant: Tenant,
    Query(query): Query<StudentsQuery>,
) -> Result<Json<StudentsResponse>, AppError> {
    require_members_view(&state, &tenant).await?;
    let certification = tree::parse_certification(&query.certification)?;
    let (page, per_page) = PageParams {
        page: query.page,
        per_page: query.per_page,
    }
    .resolve();

    let member_ids = memberships::active_user_ids(&state.db, tenant.club.id).await?;
    let (profile_rows, total) =
        profiles::page_preparing(&state.db, &member_ids, certification.code(), page, per_page)
            .await?;
    let user_ids: Vec<Uuid> = profile_rows.iter().map(|p| p.user_id).collect();
    let users = user_service::find_by_ids(&state.db, &user_ids).await?;

    let skill_ids = tree::skill_ids_of_level(&state.db, certification).await?;
    let validation_rows = validations::list_for_users(&state.db, &user_ids, &skill_ids).await?;
    let mut stages_by_user: HashMap<Uuid, HashMap<Uuid, ValidationStage>> = HashMap::new();
    for row in &validation_rows {
        if let Some(stage) = ValidationStage::parse(&row.stage) {
            stages_by_user
                .entry(row.user_id)
                .or_default()
                .insert(row.skill_id, stage);
        }
    }

    let items = user_ids
        .iter()
        .filter_map(|user_id| {
            let user = users.iter().find(|u| u.id == *user_id);
            if user.is_none() {
                tracing::error!(user_id = %user_id, "profile row with dangling user");
            }
            user
        })
        .map(|user| {
            let stages = stages_by_user.get(&user.id);
            StudentDto {
                user: UserDto::from(user),
                progress: Progress::of_stages(
                    skill_ids
                        .iter()
                        .map(|skill_id| stages.and_then(|s| s.get(skill_id).copied())),
                ),
            }
        })
        .collect();
    Ok(Json(StudentsResponse {
        level: LevelRulesDto::for_level(certification),
        page: Page::new(items, total, page, per_page),
    }))
}
