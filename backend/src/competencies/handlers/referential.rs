//! The global competency referential: levels list and tree for every
//! authenticated user, CRUD for real global admins only (product decision
//! 2026-08-19 — the tree is a platform foundation, not club data).

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::Json;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::competencies::handlers::{require_real_global_admin, LevelRulesDto, UserRefDto};
use crate::competencies::services::{documents, tree, validations};
use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::users as user_service;
use crate::pagination::{Page, PageParams};
use crate::AppState;

// ---- levels & tree (any authenticated user) ----

#[derive(Serialize)]
pub struct LevelSummaryDto {
    #[serde(flatten)]
    pub rules: LevelRulesDto,
    pub domains_count: i64,
    pub has_document: bool,
    pub page_count: Option<i32>,
}

/// Certifications having a tree and/or a document template — the entry point
/// of every competency screen. Bounded by the ffessm catalog, not paginated
/// (same precedent as /catalog/certifications).
pub async fn list_levels(
    State(state): State<AppState>,
    CurrentUser(_auth): CurrentUser,
) -> Result<Json<Vec<LevelSummaryDto>>, AppError> {
    let domain_counts = tree::level_domain_counts(&state.db).await?;
    let document_infos = documents::list_infos(&state.db).await?;

    let mut by_code: HashMap<String, LevelSummaryDto> = HashMap::new();
    for row in domain_counts {
        // Unknown codes cannot exist (writes validate), but fail safe.
        let Some(certification) = ffessm::Certification::parse(&row.certification) else {
            tracing::error!(
                code = row.certification,
                "unparseable certification in tree"
            );
            continue;
        };
        by_code.insert(
            row.certification.clone(),
            LevelSummaryDto {
                rules: LevelRulesDto::for_level(certification),
                domains_count: row.domains_count,
                has_document: false,
                page_count: None,
            },
        );
    }
    for info in document_infos {
        let entry = match by_code.get_mut(&info.certification) {
            Some(entry) => entry,
            None => {
                let Some(certification) = ffessm::Certification::parse(&info.certification) else {
                    tracing::error!(code = info.certification, "unparseable certification");
                    continue;
                };
                by_code.insert(
                    info.certification.clone(),
                    LevelSummaryDto {
                        rules: LevelRulesDto::for_level(certification),
                        domains_count: 0,
                        has_document: false,
                        page_count: None,
                    },
                );
                by_code.get_mut(&info.certification).expect("just inserted")
            }
        };
        entry.has_document = true;
        entry.page_count = Some(info.page_count);
    }

    let mut levels: Vec<LevelSummaryDto> = by_code.into_values().collect();
    // Catalog order: diver rank first, then code, for a stable UI.
    levels.sort_by(|a, b| {
        let rank = |dto: &LevelSummaryDto| {
            ffessm::Certification::parse(&dto.rules.certification)
                .and_then(|c| c.diver_rank())
                .unwrap_or(u8::MAX)
        };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.rules.certification.cmp(&b.rules.certification))
    });
    Ok(Json(levels))
}

#[derive(Deserialize)]
pub struct LevelQuery {
    pub certification: String,
}

#[derive(Serialize)]
pub struct SkillDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct ModuleDto {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i32,
    pub skills: Vec<SkillDto>,
}

#[derive(Serialize)]
pub struct DomainDto {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i32,
    pub modules: Vec<ModuleDto>,
}

#[derive(Serialize)]
pub struct TreeResponse {
    pub level: LevelRulesDto,
    pub domains: Vec<DomainDto>,
}

/// The bare referential of one level (no personal data).
pub async fn get_tree(
    State(state): State<AppState>,
    CurrentUser(_auth): CurrentUser,
    Query(query): Query<LevelQuery>,
) -> Result<Json<TreeResponse>, AppError> {
    let certification = tree::parse_certification(&query.certification)?;
    let loaded = tree::load(&state.db, certification).await?;
    let domains = loaded
        .domains
        .iter()
        .map(|domain| DomainDto {
            id: domain.id,
            name: domain.name.clone(),
            sort_order: domain.sort_order,
            modules: loaded
                .modules
                .iter()
                .filter(|m| m.domain_id == domain.id)
                .map(|module| ModuleDto {
                    id: module.id,
                    name: module.name.clone(),
                    sort_order: module.sort_order,
                    skills: loaded
                        .skills
                        .iter()
                        .filter(|s| s.module_id == module.id)
                        .map(|skill| SkillDto {
                            id: skill.id,
                            name: skill.name.clone(),
                            description: skill.description.clone(),
                            sort_order: skill.sort_order,
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect();
    Ok(Json(TreeResponse {
        level: LevelRulesDto::for_level(certification),
        domains,
    }))
}

// ---- referential CRUD (real global admin) ----

#[derive(Deserialize)]
pub struct CreateDomainRequest {
    pub certification: String,
    pub name: String,
    pub sort_order: Option<i32>,
}

pub async fn create_domain(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<CreateDomainRequest>,
) -> Result<Json<DomainDto>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&body.certification)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::create_domain(&txn, certification, &body.name, body.sort_order).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.domain.created",
        "competency_domain",
        Some(row.id),
        json!({ "certification": row.certification, "name": row.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(DomainDto {
        id: row.id,
        name: row.name,
        sort_order: row.sort_order,
        modules: vec![],
    }))
}

/// Full-replace update (fields can therefore be cleared/moved).
#[derive(Deserialize)]
pub struct UpdateDomainRequest {
    pub certification: String,
    pub name: String,
    pub sort_order: i32,
}

pub async fn update_domain(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(domain_id): Path<Uuid>,
    Json(body): Json<UpdateDomainRequest>,
) -> Result<Json<DomainDto>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&body.certification)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::find_domain(&txn, domain_id).await?;
    let row = tree::update_domain(&txn, row, certification, &body.name, body.sort_order).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.domain.updated",
        "competency_domain",
        Some(row.id),
        json!({ "certification": row.certification, "name": row.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(DomainDto {
        id: row.id,
        name: row.name,
        sort_order: row.sort_order,
        modules: vec![],
    }))
}

#[derive(Serialize)]
pub struct ImpactResponse {
    /// Member validations destroyed if this node is deleted (cascade).
    pub validations_count: u64,
}

pub async fn domain_impact(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(domain_id): Path<Uuid>,
) -> Result<Json<ImpactResponse>, AppError> {
    require_real_global_admin(&auth)?;
    tree::find_domain(&state.db, domain_id).await?;
    let skill_ids = tree::skill_ids_under_domain(&state.db, domain_id).await?;
    Ok(Json(ImpactResponse {
        validations_count: validations::count_for_skills(&state.db, &skill_ids).await?,
    }))
}

pub async fn delete_domain(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(domain_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::find_domain(&txn, domain_id).await?;
    let skill_ids = tree::skill_ids_under_domain(&txn, domain_id).await?;
    let destroyed = validations::count_for_skills(&txn, &skill_ids).await?;
    sea_orm::ModelTrait::delete(row.clone(), &txn).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.domain.deleted",
        "competency_domain",
        Some(domain_id),
        json!({ "certification": row.certification, "name": row.name, "validations_destroyed": destroyed }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "domain deleted" })))
}

#[derive(Deserialize)]
pub struct CreateModuleRequest {
    pub domain_id: Uuid,
    pub name: String,
    pub sort_order: Option<i32>,
}

pub async fn create_module(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<CreateModuleRequest>,
) -> Result<Json<ModuleDto>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::create_module(&txn, body.domain_id, &body.name, body.sort_order).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.module.created",
        "competency_module",
        Some(row.id),
        json!({ "domain_id": row.domain_id, "name": row.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(ModuleDto {
        id: row.id,
        name: row.name,
        sort_order: row.sort_order,
        skills: vec![],
    }))
}

#[derive(Deserialize)]
pub struct UpdateModuleRequest {
    pub name: String,
    pub sort_order: i32,
}

pub async fn update_module(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(module_id): Path<Uuid>,
    Json(body): Json<UpdateModuleRequest>,
) -> Result<Json<ModuleDto>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::find_module(&txn, module_id).await?;
    let row = tree::update_module(&txn, row, &body.name, body.sort_order).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.module.updated",
        "competency_module",
        Some(row.id),
        json!({ "name": row.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(ModuleDto {
        id: row.id,
        name: row.name,
        sort_order: row.sort_order,
        skills: vec![],
    }))
}

pub async fn module_impact(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(module_id): Path<Uuid>,
) -> Result<Json<ImpactResponse>, AppError> {
    require_real_global_admin(&auth)?;
    tree::find_module(&state.db, module_id).await?;
    let skill_ids = tree::skill_ids_under_modules(&state.db, &[module_id]).await?;
    Ok(Json(ImpactResponse {
        validations_count: validations::count_for_skills(&state.db, &skill_ids).await?,
    }))
}

pub async fn delete_module(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(module_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::find_module(&txn, module_id).await?;
    let skill_ids = tree::skill_ids_under_modules(&txn, &[module_id]).await?;
    let destroyed = validations::count_for_skills(&txn, &skill_ids).await?;
    sea_orm::ModelTrait::delete(row.clone(), &txn).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.module.deleted",
        "competency_module",
        Some(module_id),
        json!({ "name": row.name, "validations_destroyed": destroyed }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "module deleted" })))
}

#[derive(Deserialize)]
pub struct CreateSkillRequest {
    pub module_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

pub async fn create_skill(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<CreateSkillRequest>,
) -> Result<Json<SkillDto>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::create_skill(
        &txn,
        body.module_id,
        &body.name,
        body.description,
        body.sort_order,
    )
    .await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.skill.created",
        "competency_skill",
        Some(row.id),
        json!({ "module_id": row.module_id, "name": row.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(SkillDto {
        id: row.id,
        name: row.name,
        description: row.description,
        sort_order: row.sort_order,
    }))
}

#[derive(Deserialize)]
pub struct UpdateSkillRequest {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

pub async fn update_skill(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(skill_id): Path<Uuid>,
    Json(body): Json<UpdateSkillRequest>,
) -> Result<Json<SkillDto>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::find_skill(&txn, skill_id).await?;
    let row = tree::update_skill(&txn, row, &body.name, body.description, body.sort_order).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.skill.updated",
        "competency_skill",
        Some(row.id),
        json!({ "name": row.name }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(SkillDto {
        id: row.id,
        name: row.name,
        description: row.description,
        sort_order: row.sort_order,
    }))
}

pub async fn skill_impact(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<ImpactResponse>, AppError> {
    require_real_global_admin(&auth)?;
    tree::find_skill(&state.db, skill_id).await?;
    Ok(Json(ImpactResponse {
        validations_count: validations::count_for_skills(&state.db, &[skill_id]).await?,
    }))
}

pub async fn delete_skill(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(skill_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let row = tree::find_skill(&txn, skill_id).await?;
    let destroyed = validations::count_for_skills(&txn, &[skill_id]).await?;
    sea_orm::ModelTrait::delete(row.clone(), &txn).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.skill.deleted",
        "competency_skill",
        Some(skill_id),
        json!({ "name": row.name, "validations_destroyed": destroyed }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "skill deleted" })))
}

// ---- validation history (real global admin) ----

/// Explicit fields — `#[serde(flatten)]` breaks integer parsing in axum's
/// `Query` (serde_urlencoded buffers values as strings): `?page=2` was a 400.
#[derive(Deserialize)]
pub struct EventsQuery {
    pub user_id: Option<Uuid>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Serialize)]
pub struct EventDto {
    pub id: Uuid,
    pub user: Option<UserRefDto>,
    pub skill_id: Option<Uuid>,
    pub skill_name: String,
    pub certification: String,
    pub action: String,
    pub stage: Option<String>,
    pub previous_stage: Option<String>,
    pub validated_at: Option<chrono::NaiveDate>,
    pub performed_by: Option<UserRefDto>,
    pub performed_in_club_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

pub async fn list_events(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Page<EventDto>>, AppError> {
    require_real_global_admin(&auth)?;
    let (page, per_page) = PageParams {
        page: query.page,
        per_page: query.per_page,
    }
    .resolve();
    let (rows, total) = validations::events_page(&state.db, query.user_id, page, per_page).await?;

    let mut ids: Vec<Uuid> = rows
        .iter()
        .flat_map(|e| [Some(e.user_id), e.performed_by_user_id])
        .flatten()
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let users = user_service::find_by_ids(&state.db, &ids).await?;
    let user_ref = |id: Option<Uuid>| {
        id.and_then(|id| users.iter().find(|u| u.id == id))
            .map(|u| UserRefDto {
                id: u.id,
                first_name: u.first_name.clone(),
                last_name: u.last_name.clone(),
            })
    };

    let items = rows
        .into_iter()
        .map(|event| EventDto {
            id: event.id,
            user: user_ref(Some(event.user_id)),
            skill_id: event.skill_id,
            skill_name: event.skill_name,
            certification: event.certification,
            action: event.action,
            stage: event.stage,
            previous_stage: event.previous_stage,
            validated_at: event.validated_at,
            performed_by: user_ref(event.performed_by_user_id),
            performed_in_club_id: event.performed_in_club_id,
            notes: event.notes,
            created_at: event.created_at,
        })
        .collect();
    Ok(Json(Page::new(items, total, page, per_page)))
}
