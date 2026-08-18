//! Club-scoped access to member profiles. The profile itself is user-global;
//! what the club route adds is the VISIBILITY check: the target must be an
//! ACTIVE member of the addressed club, otherwise 404 (never reveal that the
//! user exists elsewhere).

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::handlers::UserDto;
use crate::identity::services::audit;
use crate::identity::services::users as user_service;
use crate::pagination::{Page, PageParams};
use crate::tenancy::context::Tenant;
use crate::tenancy::permissions::Permission;
use crate::tenancy::services::memberships;
use crate::users::entities::user_certifications::{PROVENANCE_RECORDED, PROVENANCE_VALIDATED};
use crate::users::handlers::{ffessm_certifications, profile_payload, ProfileResponse};
use crate::users::services::{certifications, derived, profiles, stamps, training_statuses};
use crate::AppState;

/// Read access to the club's member data: the `club.members.view` grant, OR
/// being a CERTIFIED instructor (derived E1+ from held certifications) who is
/// an active member of this club — ADR-0007 amendment (product owner,
/// 2026-08-18): "un encadrant voit les plongeurs de son club". Read-only:
/// writes keep requiring `club.members.manage`. The stagiaire MF1 status
/// grants nothing here (decision Q5: no application rights) — hence
/// `teaching_certified`, which is also age-independent (any reference
/// birthdate works when the profile has none).
async fn require_members_view(state: &AppState, tenant: &Tenant) -> Result<(), AppError> {
    if tenant.require(Permission::ClubMembersView).is_ok() {
        return Ok(());
    }
    if tenant.membership.is_some() {
        let held = ffessm_certifications(&state.db, tenant.auth.effective_user.id).await?;
        let person = ffessm::Person {
            certifications: &held,
            statuses: &[],
            birthdate: chrono::NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"),
            handisub: false,
        };
        let capacity = ffessm::supervision(&person, chrono::Utc::now().date_naive());
        if capacity.teaching_certified.is_some() {
            return Ok(());
        }
    }
    Err(AppError::Forbidden(
        "missing permission: club.members.view".to_string(),
    ))
}

/// The tenant gate for user-global data: 404 unless `user_id` is an ACTIVE
/// member of the tenant club. Every handler in this file goes through it.
async fn ensure_member(
    state: &AppState,
    tenant: &Tenant,
    user_id: Uuid,
) -> Result<users::Model, AppError> {
    let not_found = || AppError::NotFound("member not found".to_string());
    memberships::find_active(&state.db, tenant.club.id, user_id)
        .await?
        .ok_or_else(not_found)?;
    user_service::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(not_found)
}

// ---- enriched members list (badges) ----

#[derive(Debug, Serialize)]
pub struct MemberBadges {
    /// Code of the highest held diver certification (e.g. "N2"), if any.
    pub diver: Option<&'static str>,
    /// Best certified/status teaching grade ("E1".."E4").
    pub teaching: Option<String>,
    pub gp: bool,
    pub has_rifap: bool,
    /// None when no CACI date is recorded.
    pub medical_expired: Option<bool>,
    /// Birthdate missing: prerogatives cannot be derived.
    pub incomplete: bool,
}

#[derive(Debug, Serialize)]
pub struct MemberListItem {
    pub membership_id: Uuid,
    pub user: UserDto,
    pub role_id: Uuid,
    pub role_name: String,
    pub is_admin: bool,
    pub status: String,
    pub badges: MemberBadges,
}

/// Members list with diving badges — same page shape as the tenancy
/// memberships endpoint, enriched with user-module data (3 batched queries).
pub async fn list_members(
    State(state): State<AppState>,
    tenant: Tenant,
    Query(page): Query<PageParams>,
) -> Result<Json<Page<MemberListItem>>, AppError> {
    require_members_view(&state, &tenant).await?;
    let (page, per_page) = page.resolve();
    let membership_page =
        memberships::page_with_users(&state.db, tenant.club.id, page, per_page).await?;

    let user_ids: Vec<Uuid> = membership_page
        .rows
        .iter()
        .filter_map(|(m, u)| u.as_ref().map(|_| m.user_id))
        .collect();
    let profile_rows = profiles::get_many(&state.db, &user_ids).await?;
    let certification_rows = certifications::list_for_users(&state.db, &user_ids).await?;
    let status_rows = training_statuses::list_for_users(&state.db, &user_ids).await?;

    let profiles_by_user: HashMap<Uuid, _> =
        profile_rows.into_iter().map(|p| (p.user_id, p)).collect();
    let mut certs_by_user: HashMap<Uuid, Vec<_>> = HashMap::new();
    for row in certification_rows {
        certs_by_user.entry(row.user_id).or_default().push(row);
    }
    let mut statuses_by_user: HashMap<Uuid, Vec<_>> = HashMap::new();
    for row in status_rows {
        statuses_by_user.entry(row.user_id).or_default().push(row);
    }

    let today = chrono::Utc::now().date_naive();
    let empty_certs = Vec::new();
    let empty_statuses = Vec::new();
    let mut items = Vec::with_capacity(membership_page.rows.len());
    for (membership, user) in &membership_page.rows {
        let (Some(user), Some(role)) = (
            user,
            membership_page
                .roles
                .iter()
                .find(|r| r.id == membership.role_id),
        ) else {
            tracing::error!(membership_id = %membership.id, "membership with dangling user/role");
            continue;
        };
        let profile = profiles_by_user.get(&membership.user_id);
        let cert_rows = certs_by_user
            .get(&membership.user_id)
            .unwrap_or(&empty_certs);
        let held = derived::parse_certifications(cert_rows);
        let derived_view = derived::compute(
            profile,
            cert_rows,
            statuses_by_user
                .get(&membership.user_id)
                .unwrap_or(&empty_statuses),
            today,
        );
        items.push(MemberListItem {
            membership_id: membership.id,
            user: UserDto::from(user),
            role_id: role.id,
            role_name: role.name.clone(),
            is_admin: role.is_admin,
            status: membership.status.clone(),
            badges: MemberBadges {
                // Held (not exercisable) on purpose: the badge answers
                // "what card do they hold", the profile answers "what may
                // they do today".
                diver: held
                    .iter()
                    .filter(|c| c.diver_rank().is_some())
                    .max_by_key(|c| c.diver_rank())
                    .map(|c| c.code()),
                teaching: derived_view.teaching.clone(),
                gp: derived_view.gp,
                has_rifap: held.contains(&ffessm::Certification::Rifap),
                medical_expired: derived_view.medical_expired,
                incomplete: derived_view.incomplete,
            },
        });
    }
    Ok(Json(Page::new(
        items,
        membership_page.total,
        page,
        per_page,
    )))
}

// ---- individual member profile ----

pub async fn get_member_profile(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
) -> Result<Json<ProfileResponse>, AppError> {
    require_members_view(&state, &tenant).await?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    Ok(Json(profile_payload(&state.db, &user).await?))
}

/// Staff edit of a member profile — this is where the CACI date is recorded
/// (product decision 2026-08-17). Authorization-sensitive: real users only.
pub async fn put_member_profile(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
    Json(data): Json<profiles::ProfileData>,
) -> Result<Json<ProfileResponse>, AppError> {
    tenant.require_real(Permission::ClubMembersManage)?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    let held = ffessm_certifications(&state.db, user_id).await?;
    let statuses = training_statuses::list(&state.db, user_id).await?;
    let actor = tenant.actor();
    let txn = state.db.begin().await?;
    let (updated, previous) = profiles::update(&txn, user_id, data, &held, &statuses).await?;
    // Booleans only: the CACI validity date is medical-adjacent data and
    // does not belong in the audit log.
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "user.profile.updated_by_staff",
        "user",
        Some(user_id),
        json!({
            "birthdate_changed":
                previous.as_ref().and_then(|p| p.birthdate) != updated.birthdate,
            "medical_changed":
                previous.and_then(|p| p.medical_certificate_valid_until)
                    != updated.medical_certificate_valid_until,
        }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(profile_payload(&state.db, &user).await?))
}

#[derive(Deserialize)]
pub struct RecordCertificationRequest {
    pub certification: String,
    pub obtained_on: Option<chrono::NaiveDate>,
    /// True when the club VALIDATED the certification (strongest provenance),
    /// false for a simple staff-recorded entry.
    #[serde(default)]
    pub validated: bool,
}

pub async fn record_certification(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
    Json(body): Json<RecordCertificationRequest>,
) -> Result<Json<ProfileResponse>, AppError> {
    tenant.require_real(Permission::ClubMembersManage)?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    let provenance = if body.validated {
        PROVENANCE_VALIDATED
    } else {
        PROVENANCE_RECORDED
    };
    let actor = tenant.actor();
    let txn = state.db.begin().await?;
    let row = certifications::upsert(
        &txn,
        user_id,
        certifications::NewCertification {
            certification: body.certification,
            obtained_on: body.obtained_on,
            provenance,
            recorded_by_user_id: Some(tenant.auth.real_user.id),
            recorded_in_club_id: Some(tenant.club.id),
        },
    )
    .await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "user.certification.recorded",
        "user_certification",
        Some(row.id),
        json!({ "certification": row.certification, "provenance": provenance, "user_id": user_id }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(profile_payload(&state.db, &user).await?))
}

/// Staff deletion — any provenance, including validated (product decision:
/// club admins and global admins may correct validated rows).
pub async fn delete_member_certification(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id, certification_id)): Path<(String, Uuid, Uuid)>,
) -> Result<Json<ProfileResponse>, AppError> {
    tenant.require_real(Permission::ClubMembersManage)?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    let row = certifications::find_of_user(&state.db, user_id, certification_id).await?;
    let actor = tenant.actor();
    let code = row.certification.clone();
    let provenance = row.provenance.clone();
    let txn = state.db.begin().await?;
    certifications::delete(&txn, row).await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "user.certification.deleted_by_staff",
        "user_certification",
        Some(certification_id),
        json!({ "certification": code, "provenance": provenance, "user_id": user_id }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(profile_payload(&state.db, &user).await?))
}

#[derive(Deserialize)]
pub struct TrainingStatusRequest {
    /// Some(date) sets/renews the stagiaire MF1 livret, None removes it.
    pub stagiaire_mf1_valid_until: Option<chrono::NaiveDate>,
}

pub async fn set_training_status(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
    Json(body): Json<TrainingStatusRequest>,
) -> Result<Json<ProfileResponse>, AppError> {
    tenant.require_real(Permission::ClubMembersManage)?;
    let user = ensure_member(&state, &tenant, user_id).await?;
    let actor = tenant.actor();
    let txn = state.db.begin().await?;
    training_statuses::set_stagiaire_mf1(&txn, user_id, body.stagiaire_mf1_valid_until).await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "user.training_status.set",
        "user",
        Some(user_id),
        json!({ "stagiaire_mf1_valid_until": body.stagiaire_mf1_valid_until }),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(profile_payload(&state.db, &user).await?))
}

// ---- stamp (staff-managed variant) ----

pub async fn get_member_stamp(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
) -> Result<Response, AppError> {
    require_members_view(&state, &tenant).await?;
    ensure_member(&state, &tenant, user_id).await?;
    let stamp = stamps::get(&state.db, user_id)
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
    pub image_base64: String,
}

pub async fn put_member_stamp(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
    Json(body): Json<StampUploadRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tenant.require_real(Permission::ClubMembersManage)?;
    ensure_member(&state, &tenant, user_id).await?;
    let image = base64::engine::general_purpose::STANDARD
        .decode(body.image_base64.as_bytes())
        .map_err(|_| AppError::Validation("invalid base64 image".to_string()))?;
    let held = ffessm_certifications(&state.db, user_id).await?;
    let profile = profiles::get(&state.db, user_id).await?;
    let birthdate = profile.as_ref().and_then(|p| p.birthdate).ok_or_else(|| {
        AppError::Validation("the member's birthdate must be set first".to_string())
    })?;
    let person = ffessm::Person {
        certifications: &held,
        statuses: &[],
        birthdate,
        handisub: profile.as_ref().is_some_and(|p| p.handisub),
    };
    let today = chrono::Utc::now().date_naive();
    let actor = tenant.actor();
    let txn = state.db.begin().await?;
    stamps::upsert(&txn, user_id, &person, today, image).await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "user.stamp.updated_by_staff",
        "user",
        Some(user_id),
        json!({}),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "stamp updated" })))
}

pub async fn delete_member_stamp(
    State(state): State<AppState>,
    tenant: Tenant,
    Path((_slug, user_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    tenant.require_real(Permission::ClubMembersManage)?;
    ensure_member(&state, &tenant, user_id).await?;
    let actor = tenant.actor();
    let txn = state.db.begin().await?;
    stamps::delete(&txn, user_id).await?;
    audit::record(
        &txn,
        actor,
        Some(tenant.club.id),
        "user.stamp.deleted_by_staff",
        "user",
        Some(user_id),
        json!({}),
    )
    .await?;
    txn.commit().await?;
    Ok(Json(json!({ "message": "stamp deleted" })))
}
