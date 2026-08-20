//! Fixed competency rules (product owner, 2026-08-19): the validation stages
//! and the required validator grade are HARDCODED — no table, no editor.
//! Everything here is pure and keyed on the ffessm certification of the tree.

use ffessm::{Certification, SupervisionGrade};

/// The three fixed validation stages, in progression order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationStage {
    Vu,
    APerfectionner,
    Valide,
}

impl ValidationStage {
    pub const ALL: [ValidationStage; 3] = [
        ValidationStage::Vu,
        ValidationStage::APerfectionner,
        ValidationStage::Valide,
    ];

    /// Frozen storage codes (CHECK constraint in the migration).
    pub fn code(&self) -> &'static str {
        match self {
            ValidationStage::Vu => "vu",
            ValidationStage::APerfectionner => "a_perfectionner",
            ValidationStage::Valide => "valide",
        }
    }

    pub fn parse(code: &str) -> Option<ValidationStage> {
        ValidationStage::ALL.into_iter().find(|s| s.code() == code)
    }

    /// `Valide` is the final stage everywhere (the sea requirement above N1
    /// changes the meaning, not the ladder).
    pub fn is_final(&self) -> bool {
        *self == ValidationStage::Valide
    }
}

/// Above N1, the final validation necessarily happens at sea ("Validé en
/// mer"); N1 and below (PESH included) accept any environment.
pub fn sea_validation(certification: Certification) -> bool {
    certification.diver_rank().is_some_and(|rank| rank > 10)
}

/// Required teaching grade to validate a skill of this level's tree
/// (product owner, 2026-08-19): E1 below N2, E2 below N3, E3 below N4, E4
/// beyond. Levels without a diver rank (PESH, cross-track certifications)
/// sit below N2.
pub fn required_validator_grade(certification: Certification) -> SupervisionGrade {
    match certification.diver_rank() {
        Some(rank) if rank >= 40 => SupervisionGrade::E4,
        Some(rank) if rank >= 30 => SupervisionGrade::E3,
        Some(rank) if rank >= 20 => SupervisionGrade::E2,
        _ => SupervisionGrade::E1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_codes_round_trip() {
        for stage in ValidationStage::ALL {
            assert_eq!(ValidationStage::parse(stage.code()), Some(stage));
        }
        assert_eq!(ValidationStage::parse("valide_mer"), None);
        assert_eq!(ValidationStage::parse(""), None);
    }

    #[test]
    fn stage_order_is_the_progression() {
        assert!(ValidationStage::Vu < ValidationStage::APerfectionner);
        assert!(ValidationStage::APerfectionner < ValidationStage::Valide);
        assert!(ValidationStage::Valide.is_final());
        assert!(!ValidationStage::Vu.is_final());
        assert!(!ValidationStage::APerfectionner.is_final());
    }

    #[test]
    fn grade_ladder_matches_the_product_rule() {
        // E1 validates below N2.
        assert_eq!(
            required_validator_grade(Certification::N1),
            SupervisionGrade::E1
        );
        assert_eq!(
            required_validator_grade(Certification::Pesh6),
            SupervisionGrade::E1
        );
        assert_eq!(
            required_validator_grade(Certification::Pesh12),
            SupervisionGrade::E1
        );
        assert_eq!(
            required_validator_grade(Certification::Pe40),
            SupervisionGrade::E1
        );
        // E2 below N3.
        assert_eq!(
            required_validator_grade(Certification::N2),
            SupervisionGrade::E2
        );
        assert_eq!(
            required_validator_grade(Certification::Pa40),
            SupervisionGrade::E2
        );
        // E3 below N4.
        assert_eq!(
            required_validator_grade(Certification::N3),
            SupervisionGrade::E3
        );
        // E4 beyond.
        assert_eq!(
            required_validator_grade(Certification::N4Gp),
            SupervisionGrade::E4
        );
        assert_eq!(
            required_validator_grade(Certification::N5Dpe),
            SupervisionGrade::E4
        );
    }

    #[test]
    fn sea_validation_starts_above_n1() {
        assert!(!sea_validation(Certification::N1));
        assert!(!sea_validation(Certification::Pesh6));
        assert!(!sea_validation(Certification::Pesh12));
        assert!(sea_validation(Certification::N2));
        assert!(sea_validation(Certification::N3));
        assert!(sea_validation(Certification::Pa12)); // rank 11: above N1
        assert!(!sea_validation(Certification::Mf1)); // no diver rank
    }
}
