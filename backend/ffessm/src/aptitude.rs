//! Aptitudes (Code du Sport art. A322-77) and supervision grades
//! (annexe III-15b).

use serde::{Deserialize, Serialize};

/// Diver aptitude in the Code du Sport sense: what a diver is apt to DO.
/// PE = encadré (guided), PA = autonome, PESH = encadré en situation de
/// handicap (never autonomous — MFT Handisub May 2025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiverAptitude {
    /// No certification yet: guided 0-6 m only (annexe III-16b "débutants").
    Debutant,
    Pe12,
    Pe20,
    Pe40,
    Pe60,
    Pa12,
    Pa20,
    Pa40,
    Pa60,
    Pesh6,
    Pesh12,
    Pesh20,
    Pesh40,
}

impl DiverAptitude {
    /// Maximum depth of the aptitude's espace d'évolution, in meters
    /// (Code du Sport art. A322-76 / annexe III-14a).
    pub fn max_depth_m(&self) -> u8 {
        match self {
            DiverAptitude::Debutant | DiverAptitude::Pesh6 => 6,
            DiverAptitude::Pe12 | DiverAptitude::Pa12 | DiverAptitude::Pesh12 => 12,
            DiverAptitude::Pe20 | DiverAptitude::Pa20 | DiverAptitude::Pesh20 => 20,
            DiverAptitude::Pe40 | DiverAptitude::Pa40 | DiverAptitude::Pesh40 => 40,
            DiverAptitude::Pe60 | DiverAptitude::Pa60 => 60,
        }
    }

    pub fn is_autonomous(&self) -> bool {
        matches!(
            self,
            DiverAptitude::Pa12 | DiverAptitude::Pa20 | DiverAptitude::Pa40 | DiverAptitude::Pa60
        )
    }

    pub fn is_pesh(&self) -> bool {
        matches!(
            self,
            DiverAptitude::Pesh6
                | DiverAptitude::Pesh12
                | DiverAptitude::Pesh20
                | DiverAptitude::Pesh40
        )
    }

    fn family(&self) -> u8 {
        if self.is_autonomous() {
            1
        } else if self.is_pesh() {
            2
        } else {
            0 // guided (Debutant + PE)
        }
    }

    /// Whether this aptitude covers `other` (same family, deeper or equal
    /// space). We never derive PA from PE or vice-versa — strictly per
    /// annexe III-14b.
    pub fn covers(&self, other: DiverAptitude) -> bool {
        self.family() == other.family() && self.max_depth_m() >= other.max_depth_m()
    }
}

/// Gas aptitude (Code du Sport annexes III-17a / III-18a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GasAptitude {
    Air,
    /// Nitrox with O2 content <= 40 % (MFT Nitrox May 2024, annexe III-17a).
    Nitrox40,
    /// All usual nitrox mixes + pure O2 for decompression (PN-C).
    NitroxAll,
    /// Trimix down to 70 m (Trimix élémentaire).
    Pth70,
    /// Trimix down to 120 m.
    Pth120,
}

/// Supervision grades of the Code du Sport annexe III-15b (2026-01-17).
/// GP guides EXPLORATION only; E1-E4 are TEACHING grades (an E-grade also
/// guides exploration in the spaces of annexe III-16b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SupervisionGrade {
    E1,
    E2,
    E3,
    E4,
}

impl std::fmt::Display for SupervisionGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SupervisionGrade::E1 => "E1",
            SupervisionGrade::E2 => "E2",
            SupervisionGrade::E3 => "E3",
            SupervisionGrade::E4 => "E4",
        })
    }
}

impl SupervisionGrade {
    /// Maximum TEACHING depth (annexe III-16a): E1 0-6 m, E2 0-20 m,
    /// E3 0-40 m, E4 0-60 m.
    pub fn max_teaching_depth_m(&self) -> u8 {
        match self {
            SupervisionGrade::E1 => 6,
            SupervisionGrade::E2 => 20,
            SupervisionGrade::E3 => 40,
            SupervisionGrade::E4 => 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DiverAptitude::*;

    #[test]
    fn covers_is_a_depth_order_within_each_family() {
        assert!(Pe60.covers(Pe40) && Pe40.covers(Pe20) && Pe20.covers(Pe12));
        assert!(Pa60.covers(Pa40) && Pa40.covers(Pa20) && Pa20.covers(Pa12));
        assert!(Pesh40.covers(Pesh20) && Pesh20.covers(Pesh12) && Pesh12.covers(Pesh6));
        assert!(Pe12.covers(Debutant));
        // Never across families (annexe III-14b is followed strictly).
        assert!(!Pa60.covers(Pe12));
        assert!(!Pe60.covers(Pa12));
        assert!(!Pe40.covers(Pesh12));
        assert!(!Pesh40.covers(Pe12));
    }

    #[test]
    fn teaching_depths_match_annexe_iii_16a() {
        assert_eq!(SupervisionGrade::E1.max_teaching_depth_m(), 6);
        assert_eq!(SupervisionGrade::E2.max_teaching_depth_m(), 20);
        assert_eq!(SupervisionGrade::E3.max_teaching_depth_m(), 40);
        assert_eq!(SupervisionGrade::E4.max_teaching_depth_m(), 60);
        assert!(SupervisionGrade::E4 > SupervisionGrade::E1);
    }
}
