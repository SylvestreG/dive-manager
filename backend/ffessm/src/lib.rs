//! FFESSM / Code du Sport domain rules.
//!
//! Pure library: no I/O, no database. Encodes the French diving regulation —
//! certifications, aptitudes (Code du Sport art. A322-77), derived
//! prerogatives (palanquée composition, supervision, dive-director rules),
//! minimum ages and prerequisites.
//!
//! Sources: latest FFESSM Manuel de Formation Technique sheets (editions
//! noted per constant) and the Code du Sport in force since 2026-01-17
//! (annexes rewritten by the 2025-10-17 and 2025-11-24 orders). Full sourced
//! analysis: `docs/analysis/ffessm-levels.md`.
//!
//! Key principle: **holding a certification ≠ being allowed to exercise it**
//! (minors hold PA20 at 15 but exercise it at 16, an N4 examined at 17
//! supervises at 18…). See [`aptitudes`] vs [`exercisable_aptitudes`].

mod aptitude;
mod certification;
mod derive;
mod eligibility;
mod gas;
mod palanquee;
mod person;
mod supervision;

pub use aptitude::{DiverAptitude, GasAptitude, SupervisionGrade};
pub use certification::Certification;
pub use derive::{aptitudes, exercisable_aptitudes, Aptitudes, ExerciseRestriction, SCUBA_MIN_AGE};
pub use eligibility::{
    check_eligibility, covered_by, eligible_next, implied_certifications, is_redundant,
    preparable_certifications, requirements, EligibilityIssue, Requirement,
};
pub use gas::{max_o2_fraction, nitrox_mod_meters, PPO2_MAX_BAR, PPO2_MIN_BAR};
pub use palanquee::{
    validate_palanquee, PalanqueeKind, PalanqueeReport, PalanqueeViolation, PalanqueeWarning,
};
pub use person::{age_on, Person, TrainingStatus};
pub use supervision::{can_be_dp, supervision, DpContext, SupervisionCapacity};

/// Regulation snapshot this library encodes. Any regulatory change must bump
/// this string together with the affected tables and tests.
pub const REGULATION_SNAPSHOT: &str = "MFT 2025-12..2026-07 / Code du Sport consolidé 2026-01-17";
