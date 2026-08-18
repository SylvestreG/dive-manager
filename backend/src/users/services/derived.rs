//! Assembles the ffessm view of a user and computes ALL derived prerogatives
//! for the profile screen. Never persisted (business rule 9): regulation
//! changes live in the ffessm crate only.

use chrono::NaiveDate;
use ffessm::{
    can_be_dp, check_eligibility, eligible_next, exercisable_aptitudes, max_o2_fraction,
    preparable_certifications, supervision, Certification, DiverAptitude, DpContext,
    EligibilityIssue, Person, Requirement, TrainingStatus, REGULATION_SNAPSHOT,
};
use serde::Serialize;

use crate::users::entities::{user_certifications, user_profiles, user_training_statuses};

#[derive(Debug, Serialize)]
pub struct AptitudeDto {
    pub code: &'static str,
    /// "guided" | "autonomous" | "pesh"
    pub kind: &'static str,
    pub max_depth_m: u8,
}

#[derive(Debug, Serialize)]
pub struct CertRefDto {
    pub code: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Serialize)]
pub struct DerivedDto {
    /// True when the birthdate is missing: nothing can be derived.
    pub incomplete: bool,
    pub age: Option<u8>,
    pub aptitudes: Vec<AptitudeDto>,
    /// French restriction labels (from the ffessm crate).
    pub restrictions: Vec<&'static str>,
    pub gp: bool,
    pub teaching: Option<String>,
    /// Teaching grade from CERTIFICATIONS only (no dated status): drives the
    /// derived member-visibility rule (ADR-0007 amendment) and the stamp.
    pub teaching_certified: Option<String>,
    pub dp_pool: bool,
    pub dp_exploration: bool,
    pub dp_teaching: bool,
    /// Trimix/heliox limited zone (teaching <= 40 m, exploration <= 70 m):
    /// E3 minimum.
    pub dp_trimix_shallow: bool,
    /// Trimix/heliox all zones: E4 minimum.
    pub dp_trimix: bool,
    /// PN-C/PTH-120 held: required on top for mixture dives as DP.
    pub dp_mixtures_ok: bool,
    pub max_o2_percent: u8,
    /// Preparable NOW (entry requirements met — "prochaines étapes").
    pub eligible_next: Vec<CertRefDto>,
    /// Worth preparing at all (excludes held/redundant/PESH-gated/impossible;
    /// entry requirements may still be pending) — feeds the preparing picker.
    pub preparable: Vec<CertRefDto>,
    pub preparing_issues: Vec<String>,
    pub medical_expired: Option<bool>,
    pub regulation_snapshot: &'static str,
}

fn empty(incomplete: bool, medical_expired: Option<bool>) -> DerivedDto {
    DerivedDto {
        incomplete,
        age: None,
        aptitudes: vec![],
        restrictions: vec![],
        gp: false,
        teaching: None,
        teaching_certified: None,
        dp_pool: false,
        dp_exploration: false,
        dp_teaching: false,
        dp_trimix_shallow: false,
        dp_trimix: false,
        dp_mixtures_ok: false,
        max_o2_percent: 21,
        eligible_next: vec![],
        preparable: vec![],
        preparing_issues: vec![],
        medical_expired,
        regulation_snapshot: REGULATION_SNAPSHOT,
    }
}

/// Parses stored certification codes; unknown codes are LOGGED and skipped —
/// never invented, never silently fatal (business rule 4).
pub fn parse_certifications(rows: &[user_certifications::Model]) -> Vec<Certification> {
    rows.iter()
        .filter_map(|row| {
            let parsed = Certification::parse(&row.certification);
            if parsed.is_none() {
                tracing::error!(
                    code = %row.certification,
                    user_id = %row.user_id,
                    "unknown certification code in database"
                );
            }
            parsed
        })
        .collect()
}

/// ffessm view of the dated training statuses (rows with unknown status
/// codes are simply ignored — only the stagiaire MF1 exists today).
pub(crate) fn training_statuses(rows: &[user_training_statuses::Model]) -> Vec<TrainingStatus> {
    rows.iter()
        .filter(|s| s.status == user_training_statuses::STATUS_STAGIAIRE_MF1)
        .map(|s| TrainingStatus::StagiairePedagogiqueMf1 {
            livret_valid_until: s.valid_until,
        })
        .collect()
}

/// French rendering of an ffessm requirement (informational UI strings).
fn requirement_label_fr(requirement: &Requirement) -> String {
    match requirement {
        Requirement::MinAge(age) => format!("âge minimum {age} ans"),
        Requirement::MaxAge(age) => format!("âge maximum {age} ans"),
        Requirement::Holds(cert) => format!("requiert {}", cert.label_fr()),
        Requirement::HoldsForDelivery(cert) => {
            format!("requiert {} (pour la délivrance)", cert.label_fr())
        }
        Requirement::HoldsAny(options) => format!(
            "requiert l'un de : {}",
            options
                .iter()
                .map(|c| c.label_fr())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Requirement::HoldsAptitude(aptitude) => format!("aptitude {} requise", aptitude.code()),
        Requirement::TeachingGrade(grade) => format!("grade d'enseignement {grade} minimum"),
        Requirement::DiveCount { count, description } => format!("{count} {description}"),
        Requirement::SignedAptitudes(description) => (*description).to_string(),
        Requirement::ParentalAuthorizationIfMinor => {
            "autorisation du responsable légal (mineur)".to_string()
        }
        Requirement::MedicalCertificate => {
            "certificat médical (CACI) en cours de validité".to_string()
        }
        Requirement::Other(description) => (*description).to_string(),
    }
}

fn aptitude_kind(aptitude: DiverAptitude) -> &'static str {
    if aptitude.is_autonomous() {
        "autonomous"
    } else if aptitude.is_pesh() {
        "pesh"
    } else {
        "guided"
    }
}

pub fn compute(
    profile: Option<&user_profiles::Model>,
    certification_rows: &[user_certifications::Model],
    status_rows: &[user_training_statuses::Model],
    on: NaiveDate,
) -> DerivedDto {
    let medical_expired = profile
        .and_then(|p| p.medical_certificate_valid_until)
        .map(|until| until < on);
    let Some(birthdate) = profile.and_then(|p| p.birthdate) else {
        return empty(true, medical_expired);
    };

    let certifications = parse_certifications(certification_rows);
    let statuses = training_statuses(status_rows);
    let person = Person {
        certifications: &certifications,
        statuses: &statuses,
        birthdate,
        handisub: profile.is_some_and(|p| p.handisub),
    };

    let (aptitudes, restrictions) = exercisable_aptitudes(&person, on);
    let capacity = supervision(&person, on);
    let preparing_issues = profile
        .and_then(|p| p.preparing_certification.as_deref())
        .and_then(Certification::parse)
        .map(|cert| {
            check_eligibility(cert, &person, on)
                .into_iter()
                .map(|issue| match issue {
                    EligibilityIssue::Unmet(r) => {
                        format!("Non rempli : {}", requirement_label_fr(&r))
                    }
                    EligibilityIssue::NeedsManualCheck(r) => {
                        format!("À vérifier : {}", requirement_label_fr(&r))
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    DerivedDto {
        incomplete: false,
        age: Some(person.age_on(on)),
        aptitudes: aptitudes
            .diver
            .iter()
            .map(|a| AptitudeDto {
                code: a.code(),
                kind: aptitude_kind(*a),
                max_depth_m: a.max_depth_m(),
            })
            .collect(),
        restrictions: restrictions.iter().map(|r| r.label_fr()).collect(),
        gp: capacity.gp,
        teaching: capacity.teaching.map(|g| g.to_string()),
        teaching_certified: capacity.teaching_certified.map(|g| g.to_string()),
        dp_pool: can_be_dp(&person, DpContext::PoolMax6m, false, on),
        dp_exploration: can_be_dp(&person, DpContext::ExplorationAirNitrox, false, on),
        dp_teaching: can_be_dp(&person, DpContext::Teaching, false, on),
        dp_trimix_shallow: can_be_dp(&person, DpContext::TrimixShallow, false, on),
        dp_trimix: can_be_dp(&person, DpContext::TrimixAll, false, on),
        dp_mixtures_ok: person.holds_any(&[Certification::NitroxConfirme, Certification::Trimix]),
        max_o2_percent: (max_o2_fraction(&person) * 100.0).round() as u8,
        eligible_next: eligible_next(&person, on)
            .into_iter()
            .map(|c| CertRefDto {
                code: c.code(),
                label: c.label_fr(),
            })
            .collect(),
        preparable: preparable_certifications(&person, on)
            .into_iter()
            .map(|c| CertRefDto {
                code: c.code(),
                label: c.label_fr(),
            })
            .collect(),
        preparing_issues,
        medical_expired,
        regulation_snapshot: REGULATION_SNAPSHOT,
    }
}
