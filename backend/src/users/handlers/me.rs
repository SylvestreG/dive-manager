//! Self-service profile endpoints (`/me/...`). The diving profile is
//! user-global: no club in the URL, no tenant check — only the session.

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use sea_orm::TransactionTrait;
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::identity::extractors::CurrentUser;
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::sessions::AuthSession;
use crate::users::handlers::{ffessm_certifications, profile_payload, ProfileResponse};
use crate::users::services::{licence_import, profiles, stamps, training_statuses};
use crate::AppState;

/// Writes on `/me/*` mutate USER-GLOBAL data. An impersonating principal
/// (possibly club-scoped, possibly without `club.members.manage`) must not
/// gain that reach — same rule as `set_my_password` (foundation §5). Reads
/// stay allowed: impersonation is "view as".
fn require_not_impersonating(auth: &AuthSession) -> Result<(), AppError> {
    if auth.is_impersonating() {
        return Err(AppError::Forbidden(
            "action not allowed while impersonating".to_string(),
        ));
    }
    Ok(())
}

pub async fn get_my_profile(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<ProfileResponse>, AppError> {
    Ok(Json(
        profile_payload(&state.db, &auth.effective_user).await?,
    ))
}

pub async fn put_my_profile(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(mut data): Json<profiles::ProfileData>,
) -> Result<Json<ProfileResponse>, AppError> {
    require_not_impersonating(&auth)?;
    let user_id = auth.effective_user.id;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    // The CACI date is recorded by club staff (product decision 2026-08-17),
    // never self-served: keep whatever is stored — read INSIDE the
    // transaction so a concurrent staff write cannot be clobbered.
    let existing = profiles::get(&txn, user_id).await?;
    data.medical_certificate_valid_until = existing
        .as_ref()
        .and_then(|p| p.medical_certificate_valid_until);

    let held = ffessm_certifications(&txn, user_id).await?;
    let statuses = training_statuses::list(&txn, user_id).await?;
    let (updated, previous) =
        profiles::apply(&txn, user_id, existing, data, &held, &statuses).await?;
    let previous_birthdate = previous.and_then(|p| p.birthdate);
    if previous_birthdate != updated.birthdate {
        audit::record(
            &txn,
            actor,
            None,
            "user.profile.birthdate_changed",
            "user",
            Some(user_id),
            json!({ "from": previous_birthdate, "to": updated.birthdate }),
        )
        .await?;
    }
    txn.commit().await?;
    Ok(Json(
        profile_payload(&state.db, &auth.effective_user).await?,
    ))
}

// NOTE (product decision 2026-08-18): certifications are entered by club
// staff ONLY — no self-declaration endpoints. The `declared` provenance
// remains in the schema, reserved for a possible future re-opening.
// EXCEPTION (same day): each member imports their OWN official FFESSM
// licence below — the federation is the authoritative source, so the
// resulting rows carry the `imported` provenance, above `declared`.

#[derive(Deserialize)]
pub struct FfessmImportRequest {
    /// Scanned QR-code URL (preferred).
    pub url: Option<String>,
    /// Or the raw licence number + key.
    pub number: Option<String>,
    pub key: Option<String>,
}

#[derive(serde::Serialize)]
pub struct FfessmImportResponse {
    /// The full profile AFTER the import (refreshes the UI).
    pub profile: ProfileResponse,
    /// Name as printed on the licence — echoed back for confirmation.
    pub holder_name: Option<String>,
    pub licence_number: Option<String>,
    /// Certification codes written/refreshed from the licence.
    pub imported: Vec<String>,
    /// Codes left untouched because a stronger (validated) row already exists.
    pub skipped_stronger: Vec<String>,
    /// Brevet titles the licence listed that we could not map — surfaced so
    /// the club can record them by hand rather than lose them.
    pub unmapped: Vec<String>,
}

/// Import MY FFESSM licence (product decision 2026-08-18: each member brings
/// their own licence, from their profile page). The service refuses a
/// licence whose holder does not match the account (review S1).
pub async fn import_my_ffessm_licence(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<FfessmImportRequest>,
) -> Result<Json<FfessmImportResponse>, AppError> {
    require_not_impersonating(&auth)?;

    let rate_key = format!("ffessm-import:{}", auth.effective_user.id);
    if !state
        .rate
        .check(&rate_key, 30, std::time::Duration::from_secs(60))
    {
        return Err(AppError::TooManyRequests);
    }

    let outcome = licence_import::import_own_licence(
        &state,
        Actor::from(&auth),
        &auth.effective_user,
        licence_import::ImportRequest {
            url: body.url.as_deref(),
            number: body.number.as_deref(),
            key: body.key.as_deref(),
        },
    )
    .await?;

    Ok(Json(FfessmImportResponse {
        profile: profile_payload(&state.db, &auth.effective_user).await?,
        holder_name: outcome.holder_name,
        licence_number: outcome.licence_number,
        imported: outcome.imported,
        skipped_stronger: outcome.skipped_stronger,
        unmapped: outcome.unmapped,
    }))
}

pub async fn get_my_stamp(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Response, AppError> {
    let stamp = stamps::get(&state.db, auth.effective_user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("no stamp".to_string()))?;
    Ok((
        [
            (header::CONTENT_TYPE, stamp.mime_type),
            (header::CACHE_CONTROL, "private, no-store".to_string()),
        ],
        stamp.image,
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct StampUploadRequest {
    /// Base64 (standard alphabet) PNG or JPEG, <= 512 KiB decoded.
    pub image_base64: String,
}

pub async fn put_my_stamp(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
    Json(body): Json<StampUploadRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_not_impersonating(&auth)?;
    let user_id = auth.effective_user.id;
    let image = base64::engine::general_purpose::STANDARD
        .decode(body.image_base64.as_bytes())
        .map_err(|_| AppError::Validation("invalid base64 image".to_string()))?;

    let held = ffessm_certifications(&state.db, user_id).await?;
    let profile = profiles::get(&state.db, user_id).await?;
    let person = ffessm::Person {
        certifications: &held,
        statuses: &[],
        birthdate: profile.as_ref().and_then(|p| p.birthdate).ok_or_else(|| {
            AppError::Validation("set your birthdate before uploading a stamp".to_string())
        })?,
        handisub: profile.as_ref().is_some_and(|p| p.handisub),
    };
    let today = chrono::Utc::now().date_naive();
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    stamps::upsert(&txn, user_id, &person, today, image).await?;
    audit::record(
        &txn,
        actor,
        None,
        "user.stamp.updated",
        "user",
        Some(user_id),
        json!({}),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "stamp updated" })))
}

pub async fn delete_my_stamp(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_not_impersonating(&auth)?;
    let user_id = auth.effective_user.id;
    let actor = Actor::from(&auth);
    let txn = state.db.begin().await?;
    stamps::delete(&txn, user_id).await?;
    audit::record(
        &txn,
        actor,
        None,
        "user.stamp.deleted",
        "user",
        Some(user_id),
        json!({}),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "stamp deleted" })))
}
