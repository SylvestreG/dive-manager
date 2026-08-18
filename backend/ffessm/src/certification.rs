//! Certifications: what a person HOLDS (brevets, qualifications, diplomas).

use serde::{Deserialize, Serialize};

/// Every certification/qualification tracked by Dive Manager.
///
/// Deliberately absent: `PA60` — it is not a standalone qualification, it IS
/// the N3 brevet (MFT N3, Dec 2025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

    /// Map a brevet TITLE as it appears on an FFESSM licence (the
    /// `infolicencie.ffessm.fr` page) to a [`Certification`].
    ///
    /// FFESSM titles are free-form and inconsistent: accents, an optional
    /// `E1`–`E4` encadrement prefix (`"E2 - Niveau IV/GP/..."`), and compound
    /// forms (`"Plongeur Niveau 5"`, `"RIFA Plongée"`). We normalize
    /// (lowercase, accent-fold, punctuation → spaces), match an explicit
    /// table, then fall back to a conservative keyword scan for the compound
    /// encadrement lines. Returns `None` for anything we don't recognize —
    /// the caller MUST surface those to staff rather than drop them silently
    /// (ROADMAP: "fail loudly on unknown values").
    pub fn from_ffessm_label(label: &str) -> Option<Certification> {
        let norm = normalize_ffessm_label(label);
        if let Some(cert) = ffessm_label_exact(&norm) {
            return Some(cert);
        }
        // Strip a leading encadrement code ("e2 niveau iv ..." → "niveau iv
        // ...") and retry the exact table, then keyword fallback.
        let stripped = strip_encadrement_prefix(&norm);
        if stripped != norm {
            if let Some(cert) = ffessm_label_exact(stripped) {
                return Some(cert);
            }
        }
        ffessm_label_keyword(stripped)
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
            Certification::Nitrox => "Plongeur Nitrox de base",
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

/// Lowercase, fold French accents, turn every non-alphanumeric run into a
/// single space, and trim. `"E2 - Niveau IV/GP/Directeur de bassin"` →
/// `"e2 niveau iv gp directeur de bassin"`; `"Plongeur Nitrox confirmé"` →
/// `"plongeur nitrox confirme"`.
fn normalize_ffessm_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut prev_space = true; // trims leading spaces
                               // Unicode-lowercase FIRST so uppercase accents ("CONFIRMÉ") fold too.
    for ch in label.chars().flat_map(char::to_lowercase) {
        let folded = match ch {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'û' | 'ü' | 'ú' | 'ù' => 'u',
            'ÿ' => 'y',
            other => other,
        };
        if folded.is_ascii_alphanumeric() {
            out.push(folded);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Drop a leading `e1`–`e4` encadrement code from a normalized label.
fn strip_encadrement_prefix(norm: &str) -> &str {
    let bytes = norm.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'e' && bytes[1].is_ascii_digit() && bytes[2] == b' ' {
        if let b'1'..=b'4' = bytes[1] {
            return norm[3..].trim_start();
        }
    }
    norm
}

/// Exact match against known normalized FFESSM titles (and their common
/// aliases). This is the storage contract's inbound edge — keep it explicit.
fn ffessm_label_exact(norm: &str) -> Option<Certification> {
    use Certification::*;
    Some(match norm {
        // Youth pathway.
        "plongeur de bronze" | "plongeur bronze" => PlongeurBronze,
        "plongeur d argent" | "plongeur argent" => PlongeurArgent,
        "plongeur d or" | "plongeur or" => PlongeurOr,
        // Main diver pathway (canonical + "Plongeur Niveau N" + PE/PA aptitudes).
        "pe12" => Pe12,
        "niveau 1" | "plongeur niveau 1" | "n1" => N1,
        "pa12" => Pa12,
        "pe40" => Pe40,
        "pa20" => Pa20,
        "niveau 2" | "plongeur niveau 2" | "n2" => N2,
        "pa40" => Pa40,
        "pe60" => Pe60,
        "niveau 3" | "plongeur niveau 3" | "n3" => N3,
        "niveau 4" | "niveau iv" | "plongeur niveau 4" | "n4" | "guide de palanquee" => N4Gp,
        "niveau 5"
        | "niveau v"
        | "plongeur niveau 5"
        | "n5"
        | "directeur de plongee en exploration" => N5Dpe,
        // Handisub divers.
        "pesh6" | "pesh 6" => Pesh6,
        "pesh12" | "pesh 12" => Pesh12,
        "pesh20" | "pesh 20" => Pesh20,
        "pesh40" | "pesh 40" => Pesh40,
        // Gas qualifications.
        "plongeur nitrox" | "plongeur nitrox de base" | "nitrox" => Nitrox,
        "plongeur nitrox confirme" | "nitrox confirme" => NitroxConfirme,
        "plongeur trimix elementaire" | "trimix elementaire" => TrimixElementaire,
        "plongeur trimix" | "trimix" => Trimix,
        // Federal instructor pathway.
        "initiateur" => Initiateur,
        "moniteur federal 1er degre" | "moniteur federal 1 degre" | "mf1" => Mf1,
        "moniteur federal 2eme degre"
        | "moniteur federal 2e degre"
        | "moniteur federal 2 degre"
        | "mf2" => Mf2,
        "beppa" => Beppa,
        // Handisub instructors.
        "enseignant handisub eh1" | "eh1" => Eh1,
        "enseignant handisub eh2" | "eh2" => Eh2,
        // State diplomas.
        "bpjeps" | "bpjeps plongee" => Bpjeps,
        "bees1" | "bees 1" | "bees 1 plongee" => Bees1,
        "bees2" | "bees 2" | "bees 2 plongee" => Bees2,
        "dejeps cc plongee profonde"
        | "dejeps mention plongee profonde"
        | "dejeps plongee profonde et tutorat" => DejepsPlongeeProfonde,
        "dejeps" | "dejeps plongee" => Dejeps,
        "desjeps" | "desjeps plongee" => Desjeps,
        // Club-life.
        "rifa plongee" | "rifap" => Rifap,
        "tiv" | "technicien en inspection visuelle" => Tiv,
        "permis cotier" | "permis bateau cotier" | "permis bateau option cotiere" => PermisCotier,
        _ => return None,
    })
}

/// Conservative keyword fallback for compound encadrement lines like
/// `"niveau iv gp directeur de bassin"` or `"initiateur directeur de bassin"`
/// left over after the exact table misses. Checks the strongest, most
/// specific signals first so a longer title cannot collapse to a lower level.
fn ffessm_label_keyword(norm: &str) -> Option<Certification> {
    use Certification::*;
    let has = |needle: &str| norm.contains(needle);
    if has("moniteur federal 2") {
        Some(Mf2)
    } else if has("moniteur federal 1") {
        Some(Mf1)
    } else if has("niveau v") || has("niveau 5") || has("directeur de plongee en exploration") {
        Some(N5Dpe)
    } else if has("niveau iv") || has("niveau 4") || has("guide de palanquee") {
        Some(N4Gp)
    } else if has("initiateur") {
        Some(Initiateur)
    } else {
        None
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

    /// The nine brevet titles exactly as they appear on a real FFESSM
    /// licence page (`infolicencie.ffessm.fr`), including the `E1`/`E2`
    /// encadrement prefixes and the accented forms. This is the golden
    /// contract for the licence importer.
    #[test]
    fn maps_real_ffessm_licence_titles() {
        let cases: &[(&str, Certification)] = &[
            ("Plongeur Niveau 5", Certification::N5Dpe),
            ("E2 - Niveau IV/GP/Directeur de bassin", Certification::N4Gp),
            (
                "E1 - Initiateur-Directeur de bassin",
                Certification::Initiateur,
            ),
            ("Plongeur Nitrox confirmé", Certification::NitroxConfirme),
            ("Niveau 3", Certification::N3),
            ("RIFA Plongée", Certification::Rifap),
            ("Niveau 2", Certification::N2),
            ("Plongeur Nitrox", Certification::Nitrox),
            ("Niveau 1", Certification::N1),
        ];
        for (label, expected) in cases {
            assert_eq!(
                Certification::from_ffessm_label(label),
                Some(*expected),
                "FFESSM label {label:?} should map to {expected:?}"
            );
        }
    }

    #[test]
    fn every_french_label_maps_back_to_itself() {
        // The user-facing `label_fr()` is a title a licence could plausibly
        // show; each must round-trip through the FFESSM mapper.
        for cert in Certification::ALL {
            assert_eq!(
                Certification::from_ffessm_label(cert.label_fr()),
                Some(cert),
                "label_fr {:?} did not map back to {cert:?}",
                cert.label_fr()
            );
        }
    }

    #[test]
    fn unknown_titles_return_none_not_a_wrong_guess() {
        for label in [
            "",
            "   ",
            "Plongée souterraine",
            "Cours de secourisme générique",
            "Niveau 9",
            "E5 - inventé",
        ] {
            assert_eq!(
                Certification::from_ffessm_label(label),
                None,
                "{label:?} should be unmapped, not guessed"
            );
        }
    }

    #[test]
    fn label_normalization_is_accent_and_punctuation_insensitive() {
        assert_eq!(
            Certification::from_ffessm_label("NIVEAU  2"),
            Some(Certification::N2)
        );
        assert_eq!(
            Certification::from_ffessm_label("rifa plongee"),
            Some(Certification::Rifap)
        );
        assert_eq!(
            Certification::from_ffessm_label("Moniteur Fédéral 2ème degré"),
            Some(Certification::Mf2)
        );
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
