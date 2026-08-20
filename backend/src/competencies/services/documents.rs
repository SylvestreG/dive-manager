//! Level-document templates (global PDF blobs) and their stamp positions.
//! Blob discipline (user_stamps precedent): info reads NEVER select the
//! bytes; the blob is loaded only for download and generation.

use ffessm::Certification;
use sea_orm::*;
use uuid::Uuid;

use crate::competencies::entities::prelude::*;
use crate::competencies::entities::{level_document_positions, level_documents};
use crate::errors::AppError;
use crate::now;

pub const MAX_LEVEL_DOCUMENT_BYTES: usize = 5 * 1024 * 1024;

/// Blob-free projection for lists and existence checks.
#[derive(Debug, FromQueryResult)]
pub struct DocumentInfo {
    pub id: Uuid,
    pub certification: String,
    pub file_name: String,
    pub page_count: i32,
}

fn info_query() -> Select<LevelDocuments> {
    LevelDocuments::find()
        .select_only()
        .column(level_documents::Column::Id)
        .column(level_documents::Column::Certification)
        .column(level_documents::Column::FileName)
        .column(level_documents::Column::PageCount)
}

pub async fn list_infos<C: ConnectionTrait>(conn: &C) -> Result<Vec<DocumentInfo>, AppError> {
    Ok(info_query()
        .order_by_asc(level_documents::Column::Certification)
        .into_model::<DocumentInfo>()
        .all(conn)
        .await?)
}

pub async fn get_info<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
) -> Result<Option<DocumentInfo>, AppError> {
    Ok(info_query()
        .filter(level_documents::Column::Certification.eq(certification.code()))
        .into_model::<DocumentInfo>()
        .one(conn)
        .await?)
}

/// Full row including the blob — download/generation paths only.
pub async fn get_full<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
) -> Result<Option<level_documents::Model>, AppError> {
    Ok(LevelDocuments::find()
        .filter(level_documents::Column::Certification.eq(certification.code()))
        .one(conn)
        .await?)
}

/// Validates and stores a template. Re-upload replaces the bytes of the SAME
/// row, so positions (FK on the row id) survive; boxes on pages beyond the
/// new page count are reported by the editor and skipped at generation.
pub async fn upsert<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<DocumentInfo, AppError> {
    if bytes.len() > MAX_LEVEL_DOCUMENT_BYTES {
        return Err(AppError::Validation("document exceeds 5 MiB".to_string()));
    }
    if !bytes.starts_with(b"%PDF") {
        return Err(AppError::Validation(
            "document must be a PDF file".to_string(),
        ));
    }
    let page_count = super::pdf::page_count(&bytes)?;
    let file_name = sanitize_file_name(file_name);

    let existing = get_full(conn, certification).await?;
    let current = now();
    let row = match existing {
        None => {
            level_documents::ActiveModel {
                id: Set(Uuid::now_v7()),
                certification: Set(certification.code().to_string()),
                file_name: Set(file_name),
                file_data: Set(bytes),
                page_count: Set(page_count),
                created_at: Set(current),
                updated_at: Set(current),
            }
            .insert(conn)
            .await?
        }
        Some(row) => {
            let mut active: level_documents::ActiveModel = row.into();
            active.file_name = Set(file_name);
            active.file_data = Set(bytes);
            active.page_count = Set(page_count);
            active.updated_at = Set(current);
            active.update(conn).await?
        }
    };
    Ok(DocumentInfo {
        id: row.id,
        certification: row.certification,
        file_name: row.file_name,
        page_count: row.page_count,
    })
}

/// Deletes the template; positions cascade via the FK.
pub async fn delete<C: ConnectionTrait>(
    conn: &C,
    certification: Certification,
) -> Result<(), AppError> {
    let row = get_full(conn, certification)
        .await?
        .ok_or_else(|| AppError::NotFound("no document for this level".to_string()))?;
    let active: level_documents::ActiveModel = row.into();
    active.delete(conn).await?;
    Ok(())
}

/// Keeps the stored/downloaded name safe: no control characters, quotes or
/// separators (Fosse reflected the client filename verbatim into
/// Content-Disposition). Accented letters are KEPT (product decision
/// 2026-08-20) — downloads use RFC 5987 encoding for non-ASCII names.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim().to_string();
    if trimmed.is_empty() {
        "document.pdf".to_string()
    } else {
        trimmed
    }
}

// ---- stamp positions ----

pub async fn list_positions<C: ConnectionTrait>(
    conn: &C,
    level_document_id: Uuid,
) -> Result<Vec<level_document_positions::Model>, AppError> {
    Ok(LevelDocumentPositions::find()
        .filter(level_document_positions::Column::LevelDocumentId.eq(level_document_id))
        .order_by_asc(level_document_positions::Column::Page)
        .all(conn)
        .await?)
}

pub struct PositionData {
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
}

fn validate_position(document: &DocumentInfo, data: &PositionData) -> Result<(), AppError> {
    if data.page < 1 || data.page > document.page_count {
        return Err(AppError::Validation("page is out of range".to_string()));
    }
    let finite = [data.x, data.y, data.width, data.height, data.font_size]
        .iter()
        .all(|v| v.is_finite());
    if !finite || data.width <= 0.0 || data.height <= 0.0 || data.font_size <= 0.0 {
        return Err(AppError::Validation(
            "invalid position geometry".to_string(),
        ));
    }
    Ok(())
}

/// One box per (document, skill): insert or move.
pub async fn upsert_position<C: ConnectionTrait>(
    conn: &C,
    document: &DocumentInfo,
    skill_id: Uuid,
    data: PositionData,
) -> Result<level_document_positions::Model, AppError> {
    validate_position(document, &data)?;
    let existing = LevelDocumentPositions::find()
        .filter(level_document_positions::Column::LevelDocumentId.eq(document.id))
        .filter(level_document_positions::Column::SkillId.eq(skill_id))
        .one(conn)
        .await?;
    let current = now();
    let row = match existing {
        None => {
            level_document_positions::ActiveModel {
                id: Set(Uuid::now_v7()),
                level_document_id: Set(document.id),
                skill_id: Set(skill_id),
                page: Set(data.page),
                x: Set(data.x),
                y: Set(data.y),
                width: Set(data.width),
                height: Set(data.height),
                font_size: Set(data.font_size),
                created_at: Set(current),
                updated_at: Set(current),
            }
            .insert(conn)
            .await?
        }
        Some(row) => {
            let mut active: level_document_positions::ActiveModel = row.into();
            active.page = Set(data.page);
            active.x = Set(data.x);
            active.y = Set(data.y);
            active.width = Set(data.width);
            active.height = Set(data.height);
            active.font_size = Set(data.font_size);
            active.updated_at = Set(current);
            active.update(conn).await?
        }
    };
    Ok(row)
}

pub async fn delete_position<C: ConnectionTrait>(
    conn: &C,
    level_document_id: Uuid,
    skill_id: Uuid,
) -> Result<(), AppError> {
    let row = LevelDocumentPositions::find()
        .filter(level_document_positions::Column::LevelDocumentId.eq(level_document_id))
        .filter(level_document_positions::Column::SkillId.eq(skill_id))
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("position not found".to_string()))?;
    let active: level_document_positions::ActiveModel = row.into();
    active.delete(conn).await?;
    Ok(())
}
