//! Certifications: what a person HOLDS (brevets, qualifications, diplomas).

use serde::{Deserialize, Serialize};

/// Every certification/qualification tracked by Dive Manager.
///
/// Deliberately absent: `PA60` — it is not a standalone qualification, it IS
/// the N3 brevet (MFT N3, Dec 2025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Certification {
    // Youth pathway (MFT "Jeunes plongeurs", Aug 2023)
    PlongeurBronze,
    PlongeurArgent,
    PlongeurOr,
    // Main diver pathway
    Pe12,
    N1,
    Pa12,
    Pe40,
    Pa20,
    N2,
    Pa40,
    Pe60,
    N3,
    N4Gp,
    N5Dpe,
    // Handisub divers (MFT Handisub PESH, May 2025)
    Pesh6,
    Pesh12,
    Pesh20,
    Pesh40,
    // Gas qualifications (MFT Nitrox May 2024, Trimix Oct 2017)
    Nitrox,
    NitroxConfirme,
    TrimixElementaire,
    Trimix,
    // Federal instructor pathway
    Initiateur,
    Mf1,
    Mf2,
    Beppa,
    // Handisub instructors (MFT Handisub encadrants, May 2025)
    Eh1,
    Eh2,
    // State diplomas (Code du Sport annexe III-15b, 2026-01-17)
    Bpjeps,
    Bees1,
    Bees2,
    Dejeps,
    /// DEJEPS + certificat complémentaire « plongée profonde et tutorat ».
    DejepsPlongeeProfonde,
    Desjeps,
    // Club-life qualifications
    Rifap,
    Tiv,
    PermisCotier,
}

impl Certification {
    pub const ALL: [Certification; 37] = [
        Certification::PlongeurBronze,
        Certification::PlongeurArgent,
        Certification::PlongeurOr,
        Certification::Pe12,
        Certification::N1,
        Certification::Pa12,
        Certification::Pe40,
        Certification::Pa20,
        Certification::N2,
        Certification::Pa40,
        Certification::Pe60,
        Certification::N3,
        Certification::N4Gp,
        Certification::N5Dpe,
        Certification::Pesh6,
        Certification::Pesh12,
        Certification::Pesh20,
        Certification::Pesh40,
        Certification::Nitrox,
        Certification::NitroxConfirme,
        Certification::TrimixElementaire,
        Certification::Trimix,
        Certification::Initiateur,
        Certification::Mf1,
        Certification::Mf2,
        Certification::Beppa,
        Certification::Eh1,
        Certification::Eh2,
        Certification::Bpjeps,
        Certification::Bees1,
        Certification::Bees2,
        Certification::Dejeps,
        Certification::DejepsPlongeeProfonde,
        Certification::Desjeps,
        Certification::Rifap,
        Certification::Tiv,
        Certification::PermisCotier,
    ];

    /// Stable code, safe for database storage and APIs. Never rename.
    pub fn code(&self) -> &'static str {
        match self {
            Certification::PlongeurBronze => "BRONZE",
            Certification::PlongeurArgent => "ARGENT",
            Certification::PlongeurOr => "OR",
            Certification::Pe12 => "PE12",
            Certification::N1 => "N1",
            Certification::Pa12 => "PA12",
            Certification::Pe40 => "PE40",
            Certification::Pa20 => "PA20",
            Certification::N2 => "N2",
            Certification::Pa40 => "PA40",
            Certification::Pe60 => "PE60",
            Certification::N3 => "N3",
            Certification::N4Gp => "N4",
            Certification::N5Dpe => "N5",
            Certification::Pesh6 => "PESH6",
            Certification::Pesh12 => "PESH12",
            Certification::Pesh20 => "PESH20",
            Certification::Pesh40 => "PESH40",
            Certification::Nitrox => "NITROX",
            Certification::NitroxConfirme => "NITROX_C",
            Certification::TrimixElementaire => "TRIMIX_E",
            Certification::Trimix => "TRIMIX",
            Certification::Initiateur => "INITIATEUR",
            Certification::Mf1 => "MF1",
            Certification::Mf2 => "MF2",
            Certification::Beppa => "BEPPA",
            Certification::Eh1 => "EH1",
            Certification::Eh2 => "EH2",
            Certification::Bpjeps => "BPJEPS",
            Certification::Bees1 => "BEES1",
            Certification::Bees2 => "BEES2",
            Certification::Dejeps => "DEJEPS",
            Certification::DejepsPlongeeProfonde => "DEJEPS_PP",
            Certification::Desjeps => "DESJEPS",
            Certification::Rifap => "RIFAP",
            Certification::Tiv => "TIV",
            Certification::PermisCotier => "PERMIS_COTIER",
        }
    }

    pub fn parse(code: &str) -> Option<Certification> {
        Certification::ALL.into_iter().find(|c| c.code() == code)
    }

    /// French display name (user-facing).
    pub fn label_fr(&self) -> &'static str {
        match self {
            Certification::PlongeurBronze => "Plongeur de bronze",
            Certification::PlongeurArgent => "Plongeur d'argent",
            Certification::PlongeurOr => "Plongeur d'or",
            Certification::Pe12 => "PE12",
            Certification::N1 => "Niveau 1",
            Certification::Pa12 => "PA12",
            Certification::Pe40 => "PE40",
            Certification::Pa20 => "PA20",
            Certification::N2 => "Niveau 2",
            Certification::Pa40 => "PA40",
            Certification::Pe60 => "PE60",
            Certification::N3 => "Niveau 3",
            Certification::N4Gp => "Niveau 4 - Guide de palanquée",
            Certification::N5Dpe => "Niveau 5 - Directeur de plongée en exploration",
            Certification::Pesh6 => "PESH6",
            Certification::Pesh12 => "PESH12",
            Certification::Pesh20 => "PESH20",
            Certification::Pesh40 => "PESH40",
            Certification::Nitrox => "Plongeur Nitrox",
            Certification::NitroxConfirme => "Plongeur Nitrox confirmé",
            Certification::TrimixElementaire => "Plongeur Trimix élémentaire",
            Certification::Trimix => "Plongeur Trimix",
            Certification::Initiateur => "Initiateur",
            Certification::Mf1 => "Moniteur fédéral 1er degré",
            Certification::Mf2 => "Moniteur fédéral 2e degré",
            Certification::Beppa => "BEPPA",
            Certification::Eh1 => "Enseignant Handisub EH1",
            Certification::Eh2 => "Enseignant Handisub EH2",
            Certification::Bpjeps => "BPJEPS plongée",
            Certification::Bees1 => "BEES 1 plongée",
            Certification::Bees2 => "BEES 2 plongée",
            Certification::Dejeps => "DEJEPS plongée",
            Certification::DejepsPlongeeProfonde => "DEJEPS + CC plongée profonde",
            Certification::Desjeps => "DESJEPS plongée",
            Certification::Rifap => "RIFA Plongée",
            Certification::Tiv => "Technicien en inspection visuelle",
            Certification::PermisCotier => "Permis bateau (option côtière)",
        }
    }

    /// Titles delivered outside the federal club pathway (state diplomas,
    /// permis bateau, BEPPA): recorded, never "prepared" in a club.
    pub fn is_externally_delivered(&self) -> bool {
        matches!(
            self,
            Certification::Beppa
                | Certification::Bpjeps
                | Certification::Bees1
                | Certification::Bees2
                | Certification::Dejeps
                | Certification::DejepsPlongeeProfonde
                | Certification::Desjeps
                | Certification::PermisCotier
        )
    }

    /// Rank within the MAIN diver pathway, for display/sorting only
    /// ("highest diver level"). Instructor, PESH, youth, gas and club-life
    /// certifications are separate tracks and return None here.
    pub fn diver_rank(&self) -> Option<u8> {
        Some(match self {
            Certification::Pe12 => 5,
            Certification::N1 => 10,
            Certification::Pa12 => 11,
            Certification::Pe40 => 12,
            Certification::Pa20 => 13,
            Certification::N2 => 20,
            Certification::Pa40 => 21,
            Certification::Pe60 => 22,
            Certification::N3 => 30,
            Certification::N4Gp => 40,
            Certification::N5Dpe => 50,
            _ => return None,
        })
    }
}

impl std::fmt::Display for Certification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl From<Certification> for String {
    fn from(c: Certification) -> String {
        c.code().to_string()
    }
}

impl TryFrom<String> for Certification {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Certification::parse(&value).ok_or_else(|| format!("unknown certification code: {value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for cert in Certification::ALL {
            assert_eq!(Certification::parse(cert.code()), Some(cert));
            assert!(seen.insert(cert.code()), "duplicate code {}", cert.code());
            assert!(!cert.label_fr().is_empty());
        }
        assert_eq!(Certification::parse("PA60"), None, "PA60 is the N3 brevet");
    }

    /// M3: the codes are the DATABASE STORAGE contract — frozen here. Any
    /// change to this list is a data migration, not a refactor.
    #[test]
    fn codes_are_frozen_golden_list() {
        let golden: &[(&str, Certification)] = &[
            ("BRONZE", Certification::PlongeurBronze),
            ("ARGENT", Certification::PlongeurArgent),
            ("OR", Certification::PlongeurOr),
            ("PE12", Certification::Pe12),
            ("N1", Certification::N1),
            ("PA12", Certification::Pa12),
            ("PE40", Certification::Pe40),
            ("PA20", Certification::Pa20),
            ("N2", Certification::N2),
            ("PA40", Certification::Pa40),
            ("PE60", Certification::Pe60),
            ("N3", Certification::N3),
            ("N4", Certification::N4Gp),
            ("N5", Certification::N5Dpe),
            ("PESH6", Certification::Pesh6),
            ("PESH12", Certification::Pesh12),
            ("PESH20", Certification::Pesh20),
            ("PESH40", Certification::Pesh40),
            ("NITROX", Certification::Nitrox),
            ("NITROX_C", Certification::NitroxConfirme),
            ("TRIMIX_E", Certification::TrimixElementaire),
            ("TRIMIX", Certification::Trimix),
            ("INITIATEUR", Certification::Initiateur),
            ("MF1", Certification::Mf1),
            ("MF2", Certification::Mf2),
            ("BEPPA", Certification::Beppa),
            ("EH1", Certification::Eh1),
            ("EH2", Certification::Eh2),
            ("BPJEPS", Certification::Bpjeps),
            ("BEES1", Certification::Bees1),
            ("BEES2", Certification::Bees2),
            ("DEJEPS", Certification::Dejeps),
            ("DEJEPS_PP", Certification::DejepsPlongeeProfonde),
            ("DESJEPS", Certification::Desjeps),
            ("RIFAP", Certification::Rifap),
            ("TIV", Certification::Tiv),
            ("PERMIS_COTIER", Certification::PermisCotier),
        ];
        assert_eq!(golden.len(), Certification::ALL.len());
        for (code, cert) in golden {
            assert_eq!(cert.code(), *code);
        }
    }

    /// M3: serde round-trip through JSON (the API/storage representation).
    #[test]
    fn serde_json_round_trips_every_certification() {
        for cert in Certification::ALL {
            let json = serde_json::to_string(&cert).unwrap();
            assert_eq!(json, format!("\"{}\"", cert.code()));
            let back: Certification = serde_json::from_str(&json).unwrap();
            assert_eq!(back, cert);
        }
        assert!(serde_json::from_str::<Certification>("\"PA60\"").is_err());
    }

    #[test]
    fn diver_ranks_are_strictly_ordered_along_the_pathway() {
        let pathway = [
            Certification::Pe12,
            Certification::N1,
            Certification::Pa12,
            Certification::Pe40,
            Certification::Pa20,
            Certification::N2,
            Certification::Pa40,
            Certification::Pe60,
            Certification::N3,
            Certification::N4Gp,
            Certification::N5Dpe,
        ];
        let ranks: Vec<u8> = pathway.iter().map(|c| c.diver_rank().unwrap()).collect();
        assert!(ranks.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(Certification::Mf1.diver_rank(), None);
        assert_eq!(Certification::Pesh20.diver_rank(), None);
    }
}
