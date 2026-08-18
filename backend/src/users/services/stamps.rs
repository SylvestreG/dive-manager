//! Instructor stamp image (product decision 2026-08-17: instructor number
//! and stamp for E2/E3/E4 — used later to stamp level documents).

use ffessm::{Person, SupervisionGrade};
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::now;
use crate::users::entities::{prelude::*, user_stamps};

pub const MAX_STAMP_BYTES: usize = 512 * 1024;

fn detect_mime(bytes: &[u8]) -> Option<&'static str> {
    // Magic bytes only — no image decoder dependency.
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

pub async fn get<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<user_stamps::Model>, AppError> {
    Ok(UserStamps::find_by_id(user_id).one(conn).await?)
}

/// Upserts the stamp. Reserved for holders of a CERTIFIED teaching grade
/// >= E2 (the caller provides the ffessm view of the target user).
pub async fn upsert<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    person: &Person<'_>,
    on: chrono::NaiveDate,
    image: Vec<u8>,
) -> Result<(), AppError> {
    let capacity = ffessm::supervision(person, on);
    if !capacity
        .teaching_certified
        .is_some_and(|grade| grade >= SupervisionGrade::E2)
    {
        // Authorization rule, not an input problem: 403.
        return Err(AppError::Forbidden(
            "stamp is reserved for E2+ instructors".to_string(),
        ));
    }
    if image.len() > MAX_STAMP_BYTES {
        return Err(AppError::Validation(
            "stamp image exceeds 512 KiB".to_string(),
        ));
    }
    let Some(mime) = detect_mime(&image) else {
        return Err(AppError::Validation(
            "stamp must be a PNG or JPEG image".to_string(),
        ));
    };

    let existing = UserStamps::find_by_id(user_id).one(conn).await?;
    match existing {
        Some(row) => {
            let mut active: user_stamps::ActiveModel = row.into();
            active.image = Set(image);
            active.mime_type = Set(mime.to_string());
            active.updated_at = Set(now());
            active.update(conn).await?;
        }
        None => {
            user_stamps::ActiveModel {
                user_id: Set(user_id),
                image: Set(image),
                mime_type: Set(mime.to_string()),
                updated_at: Set(now()),
            }
            .insert(conn)
            .await?;
        }
    }
    Ok(())
}

pub async fn delete<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<(), AppError> {
    UserStamps::delete_by_id(user_id).exec(conn).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes_detection() {
        assert_eq!(
            detect_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00]),
            Some("image/png")
        );
        assert_eq!(detect_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(detect_mime(b"GIF89a"), None);
        assert_eq!(detect_mime(b"<svg>"), None);
        assert_eq!(detect_mime(&[]), None);
    }
}
