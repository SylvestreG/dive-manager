//! Diving profile (user-global). PUT semantics are FULL REPLACE: the client
//! always sends the complete profile it read, so clearing a field is just
//! sending null — no "absent vs null" ambiguity.

use chrono::{Datelike, NaiveDate};
use ffessm::Certification;
use sea_orm::*;
use uuid::Uuid;

use crate::errors::AppError;
use crate::now;
use crate::users::entities::{prelude::*, user_profiles, user_training_statuses};
use crate::users::services::derived;

pub const STAB_SIZES: [&str; 5] = ["XS", "S", "M", "L", "XL"];

/// Full profile payload (see module doc: full replace).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProfileData {
    pub birthdate: Option<NaiveDate>,
    pub phone: Option<String>,
    pub licence_number: Option<String>,
    pub instructor_number: Option<String>,
    pub medical_certificate_valid_until: Option<NaiveDate>,
    pub gear_regulator: bool,
    pub gear_stab: bool,
    pub gear_stab_size: Option<String>,
    pub gear_nitrox: bool,
    pub gear_second_regulator: bool,
    pub preparing_certification: Option<String>,
    #[serde(default)]
    pub handisub: bool,
}

impl ProfileData {
    /// Current stored values (or all-empty defaults when the member has no
    /// profile yet), as a starting point for a PARTIAL update — the FFESSM
    /// import patches a couple of fields and must keep everything else.
    pub fn from_existing(model: Option<&user_profiles::Model>) -> ProfileData {
        match model {
            Some(m) => ProfileData {
                birthdate: m.birthdate,
                phone: m.phone.clone(),
                licence_number: m.licence_number.clone(),
                instructor_number: m.instructor_number.clone(),
                medical_certificate_valid_until: m.medical_certificate_valid_until,
                gear_regulator: m.gear_regulator,
                gear_stab: m.gear_stab,
                gear_stab_size: m.gear_stab_size.clone(),
                gear_nitrox: m.gear_nitrox,
                gear_second_regulator: m.gear_second_regulator,
                preparing_certification: m.preparing_certification.clone(),
                handisub: m.handisub,
            },
            None => ProfileData {
                birthdate: None,
                phone: None,
                licence_number: None,
                instructor_number: None,
                medical_certificate_valid_until: None,
                gear_regulator: false,
                gear_stab: false,
                gear_stab_size: None,
                gear_nitrox: false,
                gear_second_regulator: false,
                preparing_certification: None,
                handisub: false,
            },
        }
    }
}

pub async fn get<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<user_profiles::Model>, AppError> {
    Ok(UserProfiles::find_by_id(user_id).one(conn).await?)
}

/// Batched variant for list screens (one query for a page of members).
pub async fn get_many(
    db: &DatabaseConnection,
    user_ids: &[Uuid],
) -> Result<Vec<user_profiles::Model>, AppError> {
    Ok(UserProfiles::find()
        .filter(user_profiles::Column::UserId.is_in(user_ids.to_vec()))
        .all(db)
        .await?)
}

/// One page of the profiles preparing `certification` among `user_ids`
/// (competencies students list) — one count + one fetch, ordered by user id.
pub async fn page_preparing(
    db: &DatabaseConnection,
    user_ids: &[Uuid],
    certification: &str,
    page: u64,
    per_page: u64,
) -> Result<(Vec<user_profiles::Model>, u64), AppError> {
    if user_ids.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let paginator = UserProfiles::find()
        .filter(user_profiles::Column::UserId.is_in(user_ids.to_vec()))
        .filter(user_profiles::Column::PreparingCertification.eq(certification))
        .order_by_asc(user_profiles::Column::UserId)
        .paginate(db, per_page);
    let total = paginator.num_items().await? as u64;
    let rows = paginator.fetch_page(page).await?;
    Ok((rows, total))
}

fn validate(
    data: &ProfileData,
    held_certifications: &[Certification],
    statuses: &[user_training_statuses::Model],
) -> Result<(), AppError> {
    if let Some(birthdate) = data.birthdate {
        let today = chrono::Utc::now().date_naive();
        if birthdate > today || birthdate.year_ce().1 < 1900 {
            return Err(AppError::Validation("invalid birthdate".to_string()));
        }
    }
    // The instructor number is reserved for people with a derived teaching
    // grade E1–E4, including a valid stagiaire-MF1 livret (product decision
    // 2026-08-18). Fail-closed: without a birthdate nothing is derivable, so
    // a non-null number is rejected. Clearing (null) is always allowed.
    if data.instructor_number.is_some() {
        let teaching = data.birthdate.is_some_and(|birthdate| {
            let statuses = derived::training_statuses(statuses);
            let person = ffessm::Person {
                certifications: held_certifications,
                statuses: &statuses,
                birthdate,
                handisub: data.handisub,
            };
            ffessm::supervision(&person, chrono::Utc::now().date_naive())
                .teaching
                .is_some()
        });
        if !teaching {
            return Err(AppError::Validation(
                "instructor number is reserved for instructors".to_string(),
            ));
        }
    }
    if let Some(size) = &data.gear_stab_size {
        if !STAB_SIZES.contains(&size.as_str()) {
            return Err(AppError::Validation("invalid stab size".to_string()));
        }
    }
    if let Some(code) = &data.preparing_certification {
        let cert = Certification::parse(code)
            .ok_or_else(|| AppError::Validation(format!("unknown certification: {code}")))?;
        if cert.is_externally_delivered() {
            return Err(AppError::Validation(
                "this title cannot be prepared in a club".to_string(),
            ));
        }
        if held_certifications.contains(&cert) {
            return Err(AppError::Validation(
                "certification already held".to_string(),
            ));
        }
        // Preparing something whose prerogatives are already fully covered
        // (e.g. PE40 for a N2) is meaningless — refuse it. Needs the
        // birthdate; without it the check is skipped (nothing derivable).
        if let Some(birthdate) = data.birthdate {
            let person = ffessm::Person {
                certifications: held_certifications,
                statuses: &[],
                birthdate,
                handisub: data.handisub,
            };
            if ffessm::is_redundant(cert, &person, chrono::Utc::now().date_naive()) {
                return Err(AppError::Validation(
                    "certification already covered by held certifications".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Upserts the full profile. Returns (updated, previous) so callers can
/// audit what changed (birthdate alters legal prerogatives, the CACI date is
/// staff-owned). Callers that already loaded the row (inside their
/// transaction) pass it as `existing` — no re-read, no lost-update window.
pub async fn apply<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    existing: Option<user_profiles::Model>,
    data: ProfileData,
    held_certifications: &[Certification],
    statuses: &[user_training_statuses::Model],
) -> Result<(user_profiles::Model, Option<user_profiles::Model>), AppError> {
    validate(&data, held_certifications, statuses)?;
    let previous = existing.clone();
    let current = now();

    let model = match existing {
        Some(profile) => {
            let mut active: user_profiles::ActiveModel = profile.into();
            active.birthdate = Set(data.birthdate);
            active.phone = Set(data.phone);
            active.licence_number = Set(data.licence_number);
            active.instructor_number = Set(data.instructor_number);
            active.medical_certificate_valid_until = Set(data.medical_certificate_valid_until);
            active.gear_regulator = Set(data.gear_regulator);
            active.gear_stab = Set(data.gear_stab);
            active.gear_stab_size = Set(data.gear_stab_size);
            active.gear_nitrox = Set(data.gear_nitrox);
            active.gear_second_regulator = Set(data.gear_second_regulator);
            active.preparing_certification = Set(data.preparing_certification);
            active.handisub = Set(data.handisub);
            active.updated_at = Set(current);
            active.update(conn).await?
        }
        None => {
            user_profiles::ActiveModel {
                user_id: Set(user_id),
                birthdate: Set(data.birthdate),
                phone: Set(data.phone),
                licence_number: Set(data.licence_number),
                instructor_number: Set(data.instructor_number),
                medical_certificate_valid_until: Set(data.medical_certificate_valid_until),
                gear_regulator: Set(data.gear_regulator),
                gear_stab: Set(data.gear_stab),
                gear_stab_size: Set(data.gear_stab_size),
                gear_nitrox: Set(data.gear_nitrox),
                gear_second_regulator: Set(data.gear_second_regulator),
                preparing_certification: Set(data.preparing_certification),
                handisub: Set(data.handisub),
                created_at: Set(current),
                updated_at: Set(current),
            }
            .insert(conn)
            .await?
        }
    };
    Ok((model, previous))
}

/// Convenience wrapper: load-then-apply in the caller's connection.
pub async fn update<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    data: ProfileData,
    held_certifications: &[Certification],
    statuses: &[user_training_statuses::Model],
) -> Result<(user_profiles::Model, Option<user_profiles::Model>), AppError> {
    let existing = get(conn, user_id).await?;
    apply(conn, user_id, existing, data, held_certifications, statuses).await
}

/// Values an admin intake (manual add, CSV import) brings for a person.
#[derive(Debug, Clone, Default)]
pub struct IntakeFields {
    pub birthdate: Option<NaiveDate>,
    pub phone: Option<String>,
    pub licence_number: Option<String>,
}

/// A field the intake brought that differs from the stored value — reported,
/// never applied (auth-onboarding design R22: imports never overwrite).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct FieldDiff {
    pub field: &'static str,
    pub stored: String,
    pub incoming: String,
}

/// Fills EMPTY profile fields from an admin intake and reports the fields
/// that differ. Existing values are never overwritten: the profile is
/// user-global, possibly shared with other clubs. Goes through the regular
/// profile validation (`apply`). Returns the diffs (empty = nothing to
/// review); no write when there is nothing to fill.
pub async fn fill_gaps<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    fields: IntakeFields,
) -> Result<Vec<FieldDiff>, AppError> {
    fn clean(value: Option<String>) -> Option<String> {
        value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }
    let phone = clean(fields.phone);
    let licence = clean(fields.licence_number);
    for value in [&phone, &licence].into_iter().flatten() {
        if value.len() > 30 {
            return Err(AppError::Validation("value too long".to_string()));
        }
    }

    let existing = get(conn, user_id).await?;
    let mut data = ProfileData::from_existing(existing.as_ref());
    let mut diffs = Vec::new();
    let mut changed = false;

    match (data.birthdate, fields.birthdate) {
        (None, Some(new)) => {
            data.birthdate = Some(new);
            changed = true;
        }
        (Some(old), Some(new)) if old != new => diffs.push(FieldDiff {
            field: "birthdate",
            stored: old.to_string(),
            incoming: new.to_string(),
        }),
        _ => {}
    }
    for (field, slot, incoming) in [
        ("phone", &mut data.phone, phone),
        ("licence_number", &mut data.licence_number, licence),
    ] {
        match (slot.as_ref(), incoming) {
            (None, Some(new)) => {
                *slot = Some(new);
                changed = true;
            }
            (Some(old), Some(new)) if *old != new => diffs.push(FieldDiff {
                field,
                stored: old.clone(),
                incoming: new,
            }),
            _ => {}
        }
    }

    if changed {
        let certifications = derived::parse_certifications(
            &crate::users::services::certifications::list(conn, user_id).await?,
        );
        let statuses = crate::users::services::training_statuses::list(conn, user_id).await?;
        apply(conn, user_id, existing, data, &certifications, &statuses).await?;
    }
    Ok(diffs)
}
