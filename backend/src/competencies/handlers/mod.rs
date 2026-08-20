pub mod documents;
pub mod me;
pub mod member;
pub mod referential;

use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::Serialize;
use uuid::Uuid;

use crate::competencies::domain::{required_validator_grade, sea_validation, ValidationStage};
use crate::competencies::services::{progress::Progress, tree, validations};
use crate::errors::AppError;
use crate::identity::services::sessions::AuthSession;
use crate::identity::services::users as user_service;

/// Every referential/document write requires a real, non-impersonating
/// global admin (product decision 2026-08-19: the tree is a platform
/// referential, like the platform endpoints).
pub fn require_real_global_admin(auth: &AuthSession) -> Result<(), AppError> {
    if auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    if !auth.real_user.is_global_admin {
        return Err(AppError::Forbidden(
            "global administrator required".to_string(),
        ));
    }
    Ok(())
}

/// Derived teaching grade of a user, from CERTIFICATIONS only (the stagiaire
/// MF1 status grants nothing — ADR-0007 Q5). Teaching grades are
/// age-independent, so a reference birthdate is enough.
pub async fn teaching_grade<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<ffessm::SupervisionGrade>, AppError> {
    let held = crate::users::handlers::ffessm_certifications(conn, user_id).await?;
    let person = ffessm::Person {
        certifications: &held,
        statuses: &[],
        birthdate: chrono::NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"),
        handisub: false,
    };
    Ok(ffessm::supervision(&person, chrono::Utc::now().date_naive()).teaching_certified)
}

// ---- shared response DTOs ----

#[derive(Debug, Serialize)]
pub struct UserRefDto {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Serialize)]
pub struct LevelRulesDto {
    pub certification: String,
    pub label: String,
    /// Above N1: the final validation necessarily happens at sea.
    pub sea_validation: bool,
    /// "E1".."E4" — fixed function of the level.
    pub required_validator_grade: String,
    /// The fixed ladder, in order: ["vu", "a_perfectionner", "valide"].
    pub stages: Vec<&'static str>,
}

impl LevelRulesDto {
    pub fn for_level(certification: ffessm::Certification) -> LevelRulesDto {
        LevelRulesDto {
            certification: certification.code().to_string(),
            label: certification.label_fr().to_string(),
            sea_validation: sea_validation(certification),
            required_validator_grade: required_validator_grade(certification).to_string(),
            stages: ValidationStage::ALL.iter().map(|s| s.code()).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ValidationDto {
    pub stage: String,
    pub validated_at: chrono::NaiveDate,
    pub validated_by: Option<UserRefDto>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillViewDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub validation: Option<ValidationDto>,
}

#[derive(Debug, Serialize)]
pub struct ModuleViewDto {
    pub id: Uuid,
    pub name: String,
    pub progress: Progress,
    pub skills: Vec<SkillViewDto>,
}

#[derive(Debug, Serialize)]
pub struct DomainViewDto {
    pub id: Uuid,
    pub name: String,
    pub progress: Progress,
    pub modules: Vec<ModuleViewDto>,
}

#[derive(Debug, Serialize)]
pub struct CompetencyViewDto {
    pub level: LevelRulesDto,
    pub progress: Progress,
    pub domains: Vec<DomainViewDto>,
}

/// Assembles one person's competency view for a level: tree (3 queries) +
/// their validations (1) + validator names (1) — constant whatever the tree
/// size, never per-row.
pub async fn competency_view(
    db: &DatabaseConnection,
    user_id: Uuid,
    certification: ffessm::Certification,
) -> Result<CompetencyViewDto, AppError> {
    let tree = tree::load(db, certification).await?;
    let skill_ids: Vec<Uuid> = tree.skills.iter().map(|s| s.id).collect();
    let validation_rows = validations::list_for_user(db, user_id, &skill_ids).await?;

    let validator_ids: Vec<Uuid> = {
        let mut ids: Vec<Uuid> = validation_rows
            .iter()
            .filter_map(|v| v.validated_by_user_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    let validators = user_service::find_by_ids(db, &validator_ids).await?;
    let validator_ref = |id: Option<Uuid>| {
        id.and_then(|id| validators.iter().find(|u| u.id == id))
            .map(|u| UserRefDto {
                id: u.id,
                first_name: u.first_name.clone(),
                last_name: u.last_name.clone(),
            })
    };
    let validation_by_skill: std::collections::HashMap<Uuid, _> =
        validation_rows.iter().map(|v| (v.skill_id, v)).collect();

    let mut domains = Vec::with_capacity(tree.domains.len());
    for domain in &tree.domains {
        let mut modules = Vec::new();
        for module in tree.modules.iter().filter(|m| m.domain_id == domain.id) {
            let mut skills = Vec::new();
            let mut stages = Vec::new();
            for skill in tree.skills.iter().filter(|s| s.module_id == module.id) {
                let row = validation_by_skill.get(&skill.id);
                stages.push(row.and_then(|v| ValidationStage::parse(&v.stage)));
                skills.push(SkillViewDto {
                    id: skill.id,
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    validation: row.map(|v| ValidationDto {
                        stage: v.stage.clone(),
                        validated_at: v.validated_at,
                        validated_by: validator_ref(v.validated_by_user_id),
                        notes: v.notes.clone(),
                    }),
                });
            }
            modules.push(ModuleViewDto {
                id: module.id,
                name: module.name.clone(),
                progress: Progress::of_stages(stages),
                skills,
            });
        }
        domains.push(DomainViewDto {
            id: domain.id,
            name: domain.name.clone(),
            progress: Progress::merge(modules.iter().map(|m| m.progress)),
            modules,
        });
    }

    Ok(CompetencyViewDto {
        level: LevelRulesDto::for_level(certification),
        progress: Progress::merge(domains.iter().map(|d| d.progress)),
        domains,
    })
}
