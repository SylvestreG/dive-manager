//! Level-document templates, stamp positions and filled-PDF generation.
//! Fosse shipped ALL of this without any server-side check — here template
//! and position writes are real-global-admin only, and generation goes
//! through the member-visibility gates of the calling handlers.

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::competencies::domain::ValidationStage;
use crate::competencies::handlers::require_real_global_admin;
use crate::competencies::services::{documents, pdf, tree, validations};
use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::extractors::CurrentUser;
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::users as user_service;
use crate::users::services::{certifications as user_certifications, derived};
use crate::AppState;

#[derive(Serialize)]
pub struct DocumentInfoDto {
    pub certification: String,
    pub file_name: String,
    pub page_count: i32,
}

impl From<&documents::DocumentInfo> for DocumentInfoDto {
    fn from(info: &documents::DocumentInfo) -> Self {
        DocumentInfoDto {
            certification: info.certification.clone(),
            file_name: info.file_name.clone(),
            page_count: info.page_count,
        }
    }
}

#[derive(Deserialize)]
pub struct UploadDocumentRequest {
    pub file_name: String,
    /// Raw PDF bytes, base64-encoded (the stamp-upload precedent: JSON, not
    /// multipart). The route carries a raised body limit.
    pub pdf_base64: String,
}

pub async fn upload_document(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(certification): Path<String>,
    Json(body): Json<UploadDocumentRequest>,
) -> Result<Json<DocumentInfoDto>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.pdf_base64.as_bytes())
        .map_err(|_| AppError::Validation("invalid base64 document".to_string()))?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    let info = documents::upsert(&txn, certification, &body.file_name, bytes).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.document.uploaded",
        "level_document",
        Some(info.id),
        json!({ "certification": info.certification, "file_name": info.file_name, "page_count": info.page_count }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(DocumentInfoDto::from(&info)))
}

pub async fn download_document(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(certification): Path<String>,
) -> Result<Response, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let row = documents::get_full(&state.db, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    Ok(pdf_response(row.file_data, &row.file_name))
}

pub async fn delete_document(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(certification): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    documents::delete(&txn, certification).await?;
    audit::record(
        &txn,
        actor,
        None,
        "competencies.document.deleted",
        "level_document",
        None,
        json!({ "certification": certification.code() }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "document deleted" })))
}

// ---- stamp positions ----

#[derive(Serialize)]
pub struct PositionDto {
    pub skill_id: Uuid,
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    /// True when a template re-upload left this box beyond the last page —
    /// shown by the editor, skipped by the generation.
    pub out_of_range: bool,
}

pub async fn list_positions(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(certification): Path<String>,
) -> Result<Json<Vec<PositionDto>>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let document = documents::get_info(&state.db, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    let rows = documents::list_positions(&state.db, document.id).await?;
    Ok(Json(
        rows.iter()
            .map(|row| PositionDto {
                skill_id: row.skill_id,
                page: row.page,
                x: row.x,
                y: row.y,
                width: row.width,
                height: row.height,
                font_size: row.font_size,
                out_of_range: row.page > document.page_count,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct PositionRequest {
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Defaults to 8 pt (the Fosse convention).
    pub font_size: Option<f32>,
}

impl PositionRequest {
    fn data(&self) -> documents::PositionData {
        documents::PositionData {
            page: self.page,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            font_size: self.font_size.unwrap_or(8.0),
        }
    }
}

/// Checks the skill exists AND belongs to this level's tree (a box for a
/// foreign skill would never be stamped).
async fn ensure_skill_of_level(
    state: &AppState,
    certification: ffessm::Certification,
    skill_id: Uuid,
) -> Result<(), AppError> {
    let skill = tree::find_skill(&state.db, skill_id).await?;
    if tree::certification_of_skill(&state.db, &skill).await? != certification {
        return Err(AppError::Validation(
            "skill does not belong to this level".to_string(),
        ));
    }
    Ok(())
}

pub async fn upsert_position(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path((certification, skill_id)): Path<(String, Uuid)>,
    Json(body): Json<PositionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let document = documents::get_info(&state.db, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    ensure_skill_of_level(&state, certification, skill_id).await?;
    documents::upsert_position(&state.db, &document, skill_id, body.data()).await?;
    Ok(Json(json!({ "message": "position saved" })))
}

#[derive(Deserialize)]
pub struct BatchPositionRequest {
    pub skill_id: Uuid,
    #[serde(flatten)]
    pub position: PositionRequest,
}

/// Transactional batch upsert (a partial layout is worse than none). The
/// level's tree is loaded ONCE to validate every skill — never per item.
pub async fn batch_positions(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path(certification): Path<String>,
    Json(body): Json<Vec<BatchPositionRequest>>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let document = documents::get_info(&state.db, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    let level_skills: std::collections::HashSet<Uuid> =
        tree::skill_ids_of_level(&state.db, certification)
            .await?
            .into_iter()
            .collect();
    for item in &body {
        if !level_skills.contains(&item.skill_id) {
            return Err(AppError::Validation(
                "skill does not belong to this level".to_string(),
            ));
        }
    }
    let txn = state.db.begin().await?;
    for item in &body {
        documents::upsert_position(&txn, &document, item.skill_id, item.position.data()).await?;
    }
    txn.commit().await?;
    Ok(Json(json!({ "message": "positions saved" })))
}

pub async fn delete_position(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Path((certification, skill_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_real_global_admin(&auth)?;
    let certification = tree::parse_certification(&certification)?;
    let document = documents::get_info(&state.db, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    documents::delete_position(&state.db, document.id, skill_id).await?;
    Ok(Json(json!({ "message": "position deleted" })))
}

// ---- filled-PDF generation (called by /me and club handlers, which own
// the authorization) ----

pub async fn generate_for_user(
    state: &AppState,
    user: &users::Model,
    certification: ffessm::Certification,
) -> Result<Response, AppError> {
    let document = documents::get_full(&state.db, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    let positions = documents::list_positions(&state.db, document.id).await?;

    // Only final-stage validations are stamped (business rule 11).
    let skill_ids: Vec<Uuid> = positions.iter().map(|p| p.skill_id).collect();
    let validation_rows = validations::list_for_user(&state.db, user.id, &skill_ids).await?;
    let validated: std::collections::HashMap<Uuid, _> = validation_rows
        .iter()
        .filter(|v| ValidationStage::parse(&v.stage).is_some_and(|s| s.is_final()))
        .map(|v| (v.skill_id, v))
        .collect();

    // Validator display names + derived teaching grades, batched.
    let mut validator_ids: Vec<Uuid> = validated
        .values()
        .filter_map(|v| v.validated_by_user_id)
        .collect();
    validator_ids.sort_unstable();
    validator_ids.dedup();
    let validator_users = user_service::find_by_ids(&state.db, &validator_ids).await?;
    let validator_certs = user_certifications::list_for_users(&state.db, &validator_ids).await?;
    let today = chrono::Utc::now().date_naive();
    let validator_line = |id: Option<Uuid>| -> String {
        let Some(user) = id.and_then(|id| validator_users.iter().find(|u| u.id == id)) else {
            return String::new();
        };
        let rows: Vec<_> = validator_certs
            .iter()
            .filter(|c| c.user_id == user.id)
            .cloned()
            .collect();
        let held = derived::parse_certifications(&rows);
        let person = ffessm::Person {
            certifications: &held,
            statuses: &[],
            birthdate: chrono::NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"),
            handisub: false,
        };
        let grade = ffessm::supervision(&person, today).teaching_certified;
        match grade {
            Some(grade) => format!("{} {} ({grade})", user.first_name, user.last_name),
            None => format!("{} {}", user.first_name, user.last_name),
        }
    };

    let stamps: Vec<pdf::Stamp> = positions
        .iter()
        .filter_map(|position| {
            let validation = validated.get(&position.skill_id)?;
            let date = validation.validated_at.format("%d/%m/%Y").to_string();
            let validator = validator_line(validation.validated_by_user_id);
            let lines = if position.height >= position.font_size * 2.5 && !validator.is_empty() {
                vec![date, validator]
            } else if validator.is_empty() {
                vec![date]
            } else {
                vec![format!("{date} - {validator}")]
            };
            Some(pdf::Stamp {
                page: position.page.max(1) as u32,
                x: position.x,
                y: position.y,
                width: position.width,
                height: position.height,
                font_size: position.font_size,
                lines,
            })
        })
        .collect();

    let filled = pdf::fill(&document.file_data, &stamps)?;
    let file_name = documents::sanitize_file_name(&format!(
        "Competences_{}_{}_{}.pdf",
        certification.code(),
        user.last_name.to_uppercase(),
        user.first_name
    ));
    Ok(pdf_response(filled, &file_name))
}

fn pdf_response(bytes: Vec<u8>, file_name: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_DISPOSITION, content_disposition(file_name)),
            (header::CACHE_CONTROL, "private, no-store".to_string()),
        ],
        bytes,
    )
        .into_response()
}

/// Accents are kept in file names (product decision 2026-08-20): non-ASCII
/// names travel through the RFC 5987 `filename*` parameter, with an ASCII
/// fallback for old clients. Header values must stay ASCII.
fn content_disposition(file_name: &str) -> String {
    if file_name.is_ascii() {
        return format!("attachment; filename=\"{file_name}\"");
    }
    let fallback: String = file_name
        .chars()
        .map(|c| if c.is_ascii() { c } else { '_' })
        .collect();
    let encoded: String = file_name
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' => (b as char).to_string(),
            b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::content_disposition;

    #[test]
    fn ascii_names_stay_plain() {
        assert_eq!(
            content_disposition("Competences_N2_DUPONT_Jean.pdf"),
            "attachment; filename=\"Competences_N2_DUPONT_Jean.pdf\""
        );
    }

    #[test]
    fn accented_names_get_rfc5987_encoding_and_ascii_fallback() {
        let header = content_disposition("Competences_N2_CŒUR_Léa.pdf");
        assert!(header.is_ascii(), "{header}");
        assert!(header.contains("filename=\"Competences_N2_C_UR_L_a.pdf\""));
        assert!(header.contains("filename*=UTF-8''Competences_N2_C%C5%92UR_L%C3%A9a.pdf"));
    }
}
