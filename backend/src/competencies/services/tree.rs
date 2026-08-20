//! The global competency referential: certification → domains → modules →
//! skills. Reads are constant-query (never per-row); writes are global-admin
//! only and called inside handler transactions.

use ffessm::Certification;
use sea_orm::*;
use uuid::Uuid;

use crate::competencies::entities::prelude::*;
use crate::competencies::entities::{competency_domains, competency_modules, competency_skills};
use crate::errors::AppError;
use crate::now;

/// Parses a level code against the ffessm golden list (422 otherwise).
pub fn parse_certification(code: &str) -> Result<Certification, AppError> {
    Certification::parse(code)
        .ok_or_else(|| AppError::Validation(format!("unknown certification: {code}")))
}

/// The full tree of one level: 3 queries whatever the tree size.
pub struct Tree {
    pub domains: Vec<competency_domains::Model>,
    pub modules: Vec<competency_modules::Model>,
    pub skills: Vec<competency_skills::Model>,
}

pub async fn load<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
) -> Result<Tree, AppError> {
    let domains = CompetencyDomains::find()
        .filter(competency_domains::Column::Certification.eq(certification.code()))
        .order_by_asc(competency_domains::Column::SortOrder)
        .order_by_asc(competency_domains::Column::Name)
        .all(conn)
        .await?;
    let domain_ids: Vec<Uuid> = domains.iter().map(|d| d.id).collect();
    let modules = if domain_ids.is_empty() {
        Vec::new()
    } else {
        CompetencyModules::find()
            .filter(competency_modules::Column::DomainId.is_in(domain_ids))
            .order_by_asc(competency_modules::Column::SortOrder)
            .order_by_asc(competency_modules::Column::Name)
            .all(conn)
            .await?
    };
    let module_ids: Vec<Uuid> = modules.iter().map(|m| m.id).collect();
    let skills = if module_ids.is_empty() {
        Vec::new()
    } else {
        CompetencySkills::find()
            .filter(competency_skills::Column::ModuleId.is_in(module_ids))
            .order_by_asc(competency_skills::Column::SortOrder)
            .order_by_asc(competency_skills::Column::Name)
            .all(conn)
            .await?
    };
    Ok(Tree {
        domains,
        modules,
        skills,
    })
}

/// All skill ids of a level (used to scope validation lookups).
pub async fn skill_ids_of_level<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
) -> Result<Vec<Uuid>, AppError> {
    Ok(load(conn, certification)
        .await?
        .skills
        .iter()
        .map(|s| s.id)
        .collect())
}

/// Certifications that have at least one domain, with their domain count —
/// one grouped query, no blobs.
#[derive(Debug, FromQueryResult)]
pub struct LevelDomainCount {
    pub certification: String,
    pub domains_count: i64,
}

pub async fn level_domain_counts<C: ConnectionTrait>(
    conn: &C,
) -> Result<Vec<LevelDomainCount>, AppError> {
    Ok(CompetencyDomains::find()
        .select_only()
        .column(competency_domains::Column::Certification)
        .column_as(competency_domains::Column::Id.count(), "domains_count")
        .group_by(competency_domains::Column::Certification)
        .into_model::<LevelDomainCount>()
        .all(conn)
        .await?)
}

fn validated_name(name: &str) -> Result<String, AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("name is required".to_string()));
    }
    Ok(name)
}

async fn next_domain_sort<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
) -> Result<i32, AppError> {
    let max = CompetencyDomains::find()
        .filter(competency_domains::Column::Certification.eq(certification.code()))
        .select_only()
        .column_as(competency_domains::Column::SortOrder.max(), "max")
        .into_tuple::<Option<i32>>()
        .one(conn)
        .await?
        .flatten();
    Ok(max.map_or(1, |m| m + 1))
}

pub async fn create_domain<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
    name: &str,
    sort_order: Option<i32>,
) -> Result<competency_domains::Model, AppError> {
    let sort_order = match sort_order {
        Some(v) => v,
        None => next_domain_sort(conn, certification).await?,
    };
    let created = now();
    Ok(competency_domains::ActiveModel {
        id: Set(Uuid::now_v7()),
        certification: Set(certification.code().to_string()),
        name: Set(validated_name(name)?),
        sort_order: Set(sort_order),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(conn)
    .await?)
}

pub async fn find_domain<C: ConnectionTrait>(
    conn: &C,
    id: Uuid,
) -> Result<competency_domains::Model, AppError> {
    CompetencyDomains::find_by_id(id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("domain not found".to_string()))
}

/// Full replace (PUT semantics): every field is written, so fields can be
/// cleared — Fosse's Some-only updates could never null anything.
pub async fn update_domain<C: ConnectionTrait>(
    conn: &C,
    row: competency_domains::Model,
    certification: Certification,
    name: &str,
    sort_order: i32,
) -> Result<competency_domains::Model, AppError> {
    let mut active: competency_domains::ActiveModel = row.into();
    active.certification = Set(certification.code().to_string());
    active.name = Set(validated_name(name)?);
    active.sort_order = Set(sort_order);
    active.updated_at = Set(now());
    Ok(active.update(conn).await?)
}

async fn next_module_sort<C: ConnectionTrait>(conn: &C, domain_id: Uuid) -> Result<i32, AppError> {
    let max = CompetencyModules::find()
        .filter(competency_modules::Column::DomainId.eq(domain_id))
        .select_only()
        .column_as(competency_modules::Column::SortOrder.max(), "max")
        .into_tuple::<Option<i32>>()
        .one(conn)
        .await?
        .flatten();
    Ok(max.map_or(1, |m| m + 1))
}

pub async fn create_module<C: ConnectionTrait>(
    conn: &C,
    domain_id: Uuid,
    name: &str,
    sort_order: Option<i32>,
) -> Result<competency_modules::Model, AppError> {
    find_domain(conn, domain_id).await?;
    let sort_order = match sort_order {
        Some(v) => v,
        None => next_module_sort(conn, domain_id).await?,
    };
    let created = now();
    Ok(competency_modules::ActiveModel {
        id: Set(Uuid::now_v7()),
        domain_id: Set(domain_id),
        name: Set(validated_name(name)?),
        sort_order: Set(sort_order),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(conn)
    .await?)
}

pub async fn find_module<C: ConnectionTrait>(
    conn: &C,
    id: Uuid,
) -> Result<competency_modules::Model, AppError> {
    CompetencyModules::find_by_id(id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("module not found".to_string()))
}

pub async fn update_module<C: ConnectionTrait>(
    conn: &C,
    row: competency_modules::Model,
    name: &str,
    sort_order: i32,
) -> Result<competency_modules::Model, AppError> {
    let mut active: competency_modules::ActiveModel = row.into();
    active.name = Set(validated_name(name)?);
    active.sort_order = Set(sort_order);
    active.updated_at = Set(now());
    Ok(active.update(conn).await?)
}

async fn next_skill_sort<C: ConnectionTrait>(conn: &C, module_id: Uuid) -> Result<i32, AppError> {
    let max = CompetencySkills::find()
        .filter(competency_skills::Column::ModuleId.eq(module_id))
        .select_only()
        .column_as(competency_skills::Column::SortOrder.max(), "max")
        .into_tuple::<Option<i32>>()
        .one(conn)
        .await?
        .flatten();
    Ok(max.map_or(1, |m| m + 1))
}

pub async fn create_skill<C: ConnectionTrait>(
    conn: &C,
    module_id: Uuid,
    name: &str,
    description: Option<String>,
    sort_order: Option<i32>,
) -> Result<competency_skills::Model, AppError> {
    find_module(conn, module_id).await?;
    let sort_order = match sort_order {
        Some(v) => v,
        None => next_skill_sort(conn, module_id).await?,
    };
    let created = now();
    Ok(competency_skills::ActiveModel {
        id: Set(Uuid::now_v7()),
        module_id: Set(module_id),
        name: Set(validated_name(name)?),
        description: Set(normalize_description(description)),
        sort_order: Set(sort_order),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(conn)
    .await?)
}

pub async fn find_skill<C: ConnectionTrait>(
    conn: &C,
    id: Uuid,
) -> Result<competency_skills::Model, AppError> {
    CompetencySkills::find_by_id(id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("skill not found".to_string()))
}

pub async fn update_skill<C: ConnectionTrait>(
    conn: &C,
    row: competency_skills::Model,
    name: &str,
    description: Option<String>,
    sort_order: i32,
) -> Result<competency_skills::Model, AppError> {
    let mut active: competency_skills::ActiveModel = row.into();
    active.name = Set(validated_name(name)?);
    active.description = Set(normalize_description(description));
    active.sort_order = Set(sort_order);
    active.updated_at = Set(now());
    Ok(active.update(conn).await?)
}

fn normalize_description(description: Option<String>) -> Option<String> {
    description
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
}

/// The certification of the level a skill belongs to (skill → module →
/// domain, two point reads).
pub async fn certification_of_skill<C: ConnectionTrait>(
    conn: &C,
    skill: &competency_skills::Model,
) -> Result<Certification, AppError> {
    let module = find_module(conn, skill.module_id).await?;
    let domain = find_domain(conn, module.domain_id).await?;
    parse_certification(&domain.certification)
}

/// Skill ids under one node of the tree (for cascade-impact counts).
pub async fn skill_ids_under_domain<C: ConnectionTrait>(
    conn: &C,
    domain_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let module_ids: Vec<Uuid> = CompetencyModules::find()
        .filter(competency_modules::Column::DomainId.eq(domain_id))
        .all(conn)
        .await?
        .iter()
        .map(|m| m.id)
        .collect();
    skill_ids_under_modules(conn, &module_ids).await
}

pub async fn skill_ids_under_modules<C: ConnectionTrait>(
    conn: &C,
    module_ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    if module_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(CompetencySkills::find()
        .filter(competency_skills::Column::ModuleId.is_in(module_ids.to_vec()))
        .all(conn)
        .await?
        .iter()
        .map(|s| s.id)
        .collect())
}
