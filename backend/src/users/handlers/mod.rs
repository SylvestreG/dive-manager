pub mod catalog;
pub mod me;
pub mod member;

use chrono::NaiveDate;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::identity::entities::users;
use crate::identity::handlers::UserDto;
use crate::users::entities::{prelude::*, user_profiles};
use crate::users::services::{certifications, derived, profiles, training_statuses};
use sea_orm::{ConnectionTrait, EntityTrait, PaginatorTrait};

#[derive(Debug, Serialize)]
pub struct ProfileFieldsDto {
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
    pub handisub: bool,
}

impl From<Option<&user_profiles::Model>> for ProfileFieldsDto {
    fn from(profile: Option<&user_profiles::Model>) -> Self {
        match profile {
            Some(p) => ProfileFieldsDto {
                birthdate: p.birthdate,
                phone: p.phone.clone(),
                licence_number: p.licence_number.clone(),
                instructor_number: p.instructor_number.clone(),
                medical_certificate_valid_until: p.medical_certificate_valid_until,
                gear_regulator: p.gear_regulator,
                gear_stab: p.gear_stab,
                gear_stab_size: p.gear_stab_size.clone(),
                gear_nitrox: p.gear_nitrox,
                gear_second_regulator: p.gear_second_regulator,
                preparing_certification: p.preparing_certification.clone(),
                handisub: p.handisub,
            },
            None => ProfileFieldsDto {
                birthdate: None,
                phone: None,
                licence_number: None,
                instructor_number: None,
                medical_certificate_valid_until: None,
                gear_regulator: true,
                gear_stab: true,
                gear_stab_size: None,
                gear_nitrox: false,
                gear_second_regulator: false,
                preparing_certification: None,
                handisub: false,
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CertificationDto {
    pub id: Uuid,
    pub code: String,
    pub label: String,
    pub obtained_on: Option<NaiveDate>,
    pub provenance: String,
}

#[derive(Debug, Serialize)]
pub struct TrainingStatusDto {
    pub status: String,
    pub valid_until: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub user: UserDto,
    pub profile: ProfileFieldsDto,
    pub certifications: Vec<CertificationDto>,
    pub training_statuses: Vec<TrainingStatusDto>,
    pub derived: derived::DerivedDto,
    pub has_stamp: bool,
}

/// Assembles the full profile payload (4 point queries + pure computation).
pub async fn profile_payload(
    db: &DatabaseConnection,
    user: &users::Model,
) -> Result<ProfileResponse, AppError> {
    let profile = profiles::get(db, user.id).await?;
    let certification_rows = certifications::list(db, user.id).await?;
    let status_rows = training_statuses::list(db, user.id).await?;
    // Existence only — never pull the image blob on the profile path.
    let has_stamp = UserStamps::find_by_id(user.id).count(db).await? > 0;

    let today = chrono::Utc::now().date_naive();
    let derived = derived::compute(profile.as_ref(), &certification_rows, &status_rows, today);

    Ok(ProfileResponse {
        user: UserDto::from(user),
        profile: ProfileFieldsDto::from(profile.as_ref()),
        certifications: certification_rows
            .into_iter()
            .map(|row| CertificationDto {
                label: ffessm::Certification::parse(&row.certification)
                    .map(|c| c.label_fr().to_string())
                    .unwrap_or_else(|| row.certification.clone()),
                id: row.id,
                code: row.certification,
                obtained_on: row.obtained_on,
                provenance: row.provenance,
            })
            .collect(),
        training_statuses: status_rows
            .into_iter()
            .map(|row| TrainingStatusDto {
                status: row.status,
                valid_until: row.valid_until,
            })
            .collect(),
        derived,
        has_stamp,
    })
}

/// Builds the ffessm person view for permission-style checks (stamps).
pub async fn ffessm_certifications<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<ffessm::Certification>, AppError> {
    let rows = certifications::list(conn, user_id).await?;
    Ok(derived::parse_certifications(&rows))
}
