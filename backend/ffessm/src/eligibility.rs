//! Matrix 4: prerequisites and minimum ages per certification (MFT sheets in
//! force — editions cited per row) and progression helper.

use chrono::NaiveDate;

use crate::derive::aptitudes;
use crate::{Certification, DiverAptitude, Person, SupervisionGrade};

/// One entry requirement of a certification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Minimum age at delivery (MFT).
    MinAge(u8),
    /// Maximum age (youth pathway is 8-14 — MFT Jeunes plongeurs).
    MaxAge(u8),
    Holds(Certification),
    HoldsAny(&'static [Certification]),
    /// Requires a derived diver aptitude (e.g. Initiateur: "PA20 minimum").
    HoldsAptitude(DiverAptitude),
    /// Requires an exercisable teaching grade.
    TeachingGrade(SupervisionGrade),
    /// Attested dive count — NOT machine-checkable by this library.
    DiveCount {
        count: u16,
        description: &'static str,
    },
    /// Examination aptitudes signed by an instructor (9-month validity).
    SignedAptitudes(&'static str),
    /// Parental authorization when the candidate is a minor.
    ParentalAuthorizationIfMinor,
    /// CACI < 1 year (FFESSM medical rules).
    MedicalCertificate,
    /// Human-checked condition.
    Other(&'static str),
}

/// Entry requirements per certification. Every row is sourced in
/// `docs/analysis/ffessm-levels.md` §3-4 (MFT editions Dec 2025 - Jul 2026).
pub fn requirements(cert: Certification) -> &'static [Requirement] {
    use Certification as C;
    use Requirement as R;
    match cert {
        // Youth (MFT Jeunes plongeurs, Aug 2023)
        C::PlongeurBronze | C::PlongeurArgent => &[
            R::MinAge(8),
            R::MaxAge(14),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        C::PlongeurOr => &[
            R::MinAge(10),
            R::MaxAge(14),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        // PE12 (MFT May 2024): no licence, no CACI.
        C::Pe12 => &[R::MinAge(10), R::ParentalAuthorizationIfMinor],
        // N1 (MFT Dec 2025)
        C::N1 => &[
            R::MinAge(12),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        // PA12 (MFT May 2026)
        C::Pa12 => &[
            R::MinAge(15),
            R::Holds(C::N1),
            R::DiveCount {
                count: 4,
                description: "plongées attestées en milieu naturel",
            },
            R::MedicalCertificate,
        ],
        // PE40 / PA20: N2 modules (MFT N2, May 2026)
        C::Pe40 => &[
            R::MinAge(14),
            R::Holds(C::N1),
            R::DiveCount {
                count: 4,
                description: "plongées attestées en milieu naturel",
            },
            R::MedicalCertificate,
        ],
        C::Pa20 => &[
            R::MinAge(15),
            R::Holds(C::N1),
            R::DiveCount {
                count: 4,
                description: "plongées attestées en milieu naturel",
            },
            R::MedicalCertificate,
        ],
        C::N2 => &[
            R::MinAge(15),
            R::Holds(C::N1),
            R::DiveCount {
                count: 4,
                description: "plongées attestées en milieu naturel",
            },
            R::MedicalCertificate,
        ],
        // PA40 / PE60: N3 modules (MFT N3, Dec 2025)
        C::Pa40 => &[R::MinAge(16), R::Holds(C::N2), R::MedicalCertificate],
        C::Pe60 => &[
            R::MinAge(17),
            R::HoldsAny(&[C::Pe40, C::N2]),
            R::DiveCount {
                count: 20,
                description: "plongées en milieu naturel dont 5 entre 35 et 40 m",
            },
            R::MedicalCertificate,
        ],
        C::N3 => &[
            R::MinAge(17),
            R::HoldsAny(&[C::N2, C::Pa40]),
            R::Holds(C::Rifap),
            R::MedicalCertificate,
        ],
        // N4-GP (MFT May 2026): 17 at the exam, N3 again required (the 2023
        // PA40 bridge was removed).
        C::N4Gp => &[
            R::MinAge(17),
            R::Holds(C::N3),
            R::Holds(C::Rifap),
            R::SignedAptitudes("aptitudes A1-A4 signées par un E3 minimum, validité 9 mois"),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        // N5-DPE (MFT May 2026)
        C::N5Dpe => &[
            R::MinAge(18),
            R::Holds(C::N4Gp),
            R::Holds(C::Rifap),
            R::DiveCount {
                count: 15,
                description: "plongées en situation de guide de palanquée",
            },
            R::MedicalCertificate,
        ],
        // Handisub divers (MFT May 2025)
        C::Pesh6 => &[
            R::MinAge(8),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        C::Pesh12 => &[
            R::MinAge(10),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        C::Pesh20 => &[
            R::MinAge(12),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        C::Pesh40 => &[
            R::MinAge(18),
            R::Holds(C::Pesh20),
            R::DiveCount {
                count: 10,
                description: "plongées en zone des 20 m signées EH1 minimum",
            },
            R::MedicalCertificate,
        ],
        // Gas (MFT Nitrox May 2024; PESH20 opens elementary nitrox)
        C::Nitrox => &[
            R::MinAge(12),
            R::HoldsAny(&[C::N1, C::Pesh20]),
            R::MedicalCertificate,
        ],
        C::NitroxConfirme => &[
            R::MinAge(15),
            R::Holds(C::N2),
            R::Holds(C::Nitrox),
            R::MedicalCertificate,
        ],
        // Trimix (MFT Oct 2017)
        C::TrimixElementaire => &[
            R::MinAge(18),
            R::Holds(C::N3),
            R::Holds(C::NitroxConfirme),
            R::DiveCount {
                count: 15,
                description: "plongées à plus de 40 m",
            },
            R::MedicalCertificate,
        ],
        C::Trimix => &[
            R::MinAge(18),
            R::Holds(C::TrimixElementaire),
            R::DiveCount {
                count: 10,
                description:
                    "plongées trimix dont 5 entre 50 et 70 m et 5 avec décompression nitrox/O2",
            },
            R::MedicalCertificate,
        ],
        // Initiateur (MFT Dec 2025): 16 at the exam, PA20 minimum.
        C::Initiateur => &[
            R::MinAge(16),
            R::HoldsAptitude(DiverAptitude::Pa20),
            R::Holds(C::Rifap),
            R::DiveCount {
                count: 12,
                description: "plongées après l'obtention du PA20 (dispense si N3, GP-N4 ou DP-N5)",
            },
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        // MF1 (MFT Jul 2026)
        C::Mf1 => &[
            R::MinAge(18),
            R::HoldsAny(&[C::N4Gp, C::Bpjeps]),
            R::Holds(C::Rifap),
            R::Holds(C::PermisCotier),
            R::MedicalCertificate,
        ],
        // MF2 (MFT May 2026): 20 at the exam, Nitrox Confirmé now required.
        C::Mf2 => &[
            R::MinAge(20),
            R::Holds(C::Mf1),
            R::Holds(C::Rifap),
            R::Holds(C::NitroxConfirme),
            R::Holds(C::PermisCotier),
            R::SignedAptitudes("aptitudes A1-A5 signées par un E4, validité 9 mois"),
            R::Other("MF1 depuis 12 mois minimum"),
            R::MedicalCertificate,
        ],
        // Handisub instructors (MFT May 2025)
        C::Eh1 => &[
            R::TeachingGrade(SupervisionGrade::E1),
            R::DiveCount {
                count: 5,
                description: "plongées d'enseignement depuis l'obtention du grade",
            },
            R::MedicalCertificate,
        ],
        C::Eh2 => &[
            R::MinAge(18),
            R::TeachingGrade(SupervisionGrade::E2),
            R::Holds(C::Eh1),
            R::Holds(C::Rifap),
            R::DiveCount {
                count: 5,
                description: "encadrements Handisub",
            },
            R::MedicalCertificate,
        ],
        // Club life
        C::Rifap => &[R::ParentalAuthorizationIfMinor],
        C::Tiv => &[R::MinAge(18), R::Other("présenté par le président du club")],
        // State-delivered titles: outside federal eligibility.
        C::Beppa
        | C::Bpjeps
        | C::Bees1
        | C::Bees2
        | C::Dejeps
        | C::DejepsPlongeeProfonde
        | C::Desjeps
        | C::PermisCotier => &[R::Other("titre délivré par l'État")],
    }
}

/// Outcome of checking one requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EligibilityIssue {
    /// Machine-checked and NOT met.
    Unmet(Requirement),
    /// Cannot be verified by the library (dive counts, medical, paperwork) —
    /// the club must confirm manually.
    NeedsManualCheck(Requirement),
}

/// Checks `person` against the entry requirements of `cert` on date `on`.
/// An empty result means "machine-checkable requirements all met, nothing to
/// confirm manually".
pub fn check_eligibility(
    cert: Certification,
    person: &Person,
    on: NaiveDate,
) -> Vec<EligibilityIssue> {
    use Requirement as R;
    let rows = requirements(cert);
    // Compute derivations only when a row needs them.
    let derived = rows
        .iter()
        .any(|r| matches!(r, R::HoldsAptitude(_)))
        .then(|| aptitudes(person, on));
    let capacity = rows
        .iter()
        .any(|r| matches!(r, R::TeachingGrade(_)))
        .then(|| crate::supervision::supervision(person, on));
    let mut issues = Vec::new();
    // Parental authorization is a blanket rule for minors (MFT generalities,
    // Jan 2025) — enforced globally, whatever the certification row says.
    if person.is_minor_on(on) {
        issues.push(EligibilityIssue::NeedsManualCheck(
            Requirement::ParentalAuthorizationIfMinor,
        ));
    }
    for requirement in rows {
        match requirement {
            R::MinAge(age) => {
                if person.age_on(on) < *age {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            R::MaxAge(age) => {
                if person.age_on(on) > *age {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            R::Holds(needed) => {
                if !person.holds(*needed) {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            R::HoldsAny(options) => {
                if !person.holds_any(options) {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            R::HoldsAptitude(aptitude) => {
                if !derived.as_ref().is_some_and(|d| d.has(*aptitude)) {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            R::TeachingGrade(grade) => {
                let met = capacity
                    .as_ref()
                    .is_some_and(|c| c.teaching.is_some_and(|t| t >= *grade));
                if !met {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            // Handled globally above.
            R::ParentalAuthorizationIfMinor => {}
            R::DiveCount { .. } | R::SignedAptitudes(_) | R::MedicalCertificate | R::Other(_) => {
                issues.push(EligibilityIssue::NeedsManualCheck(*requirement));
            }
        }
    }
    issues
}

/// Certifications the person could start preparing now: not held, and no
/// machine-checkable requirement unmet (manual checks remain to confirm).
pub fn eligible_next(person: &Person, on: NaiveDate) -> Vec<Certification> {
    Certification::ALL
        .into_iter()
        .filter(|cert| !cert.is_externally_delivered())
        .filter(|cert| !person.holds(*cert))
        .filter(|cert| {
            check_eligibility(*cert, person, on)
                .iter()
                .all(|issue| matches!(issue, EligibilityIssue::NeedsManualCheck(_)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Certification as C;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn on() -> NaiveDate {
        d(2026, 8, 17)
    }
    fn person(certs: &'static [C], birth_year: i32) -> Person<'static> {
        Person {
            certifications: certs,
            statuses: &[],
            birthdate: d(birth_year, 1, 1),
        }
    }
    fn unmet(cert: C, p: &Person) -> Vec<Requirement> {
        check_eligibility(cert, p, on())
            .into_iter()
            .filter_map(|i| match i {
                EligibilityIssue::Unmet(r) => Some(r),
                EligibilityIssue::NeedsManualCheck(_) => None,
            })
            .collect()
    }

    #[test]
    fn minimum_ages_match_the_mft_in_force() {
        // (certification, prerequisites making everything else pass, min age)
        let cases: &[(C, &'static [C], u8)] = &[
            (C::PlongeurBronze, &[], 8),
            (C::PlongeurOr, &[], 10),
            (C::Pe12, &[], 10),
            (C::N1, &[], 12),
            (C::Nitrox, &[C::N1], 12),
            (C::Pe40, &[C::N1], 14),
            (C::Pa12, &[C::N1], 15),
            (C::Pa20, &[C::N1], 15),
            (C::N2, &[C::N1], 15),
            (C::NitroxConfirme, &[C::N2, C::Nitrox], 15),
            (C::Pa40, &[C::N2], 16),
            (C::Initiateur, &[C::N2, C::Rifap], 16),
            (C::Pe60, &[C::N2], 17),
            (C::N3, &[C::N2, C::Rifap], 17),
            (C::N4Gp, &[C::N3, C::Rifap], 17),
            (C::N5Dpe, &[C::N4Gp, C::Rifap], 18),
            (C::Pesh40, &[C::Pesh20], 18),
            (C::TrimixElementaire, &[C::N3, C::NitroxConfirme], 18),
            (C::Mf1, &[C::N4Gp, C::Rifap, C::PermisCotier], 18),
            (
                C::Mf2,
                &[C::Mf1, C::Rifap, C::NitroxConfirme, C::PermisCotier],
                20,
            ),
            (C::Tiv, &[], 18),
        ];
        for (cert, prereqs, min_age) in cases {
            // The day before the minimum-age birthday: MinAge unmet.
            let too_young = person(prereqs, 2026 - (*min_age as i32) + 1);
            assert!(
                unmet(*cert, &too_young).contains(&Requirement::MinAge(*min_age)),
                "{cert:?} should require age {min_age}",
            );
            // At the minimum age with prerequisites: nothing unmet.
            let old_enough = person(prereqs, 2026 - (*min_age as i32));
            assert!(
                unmet(*cert, &old_enough).is_empty(),
                "{cert:?} at {min_age} with {prereqs:?}: {:?}",
                unmet(*cert, &old_enough),
            );
        }
    }

    #[test]
    fn prerequisite_chains_are_enforced() {
        let nobody = person(&[], 1990);
        assert!(unmet(C::Pa12, &nobody).contains(&Requirement::Holds(C::N1)));
        assert!(unmet(C::N3, &nobody).contains(&Requirement::HoldsAny(&[C::N2, C::Pa40])));
        assert!(unmet(C::N3, &nobody).contains(&Requirement::Holds(C::Rifap)));
        // N4 requires N3 again (the 2023 PA40 bridge was removed).
        let pa40_only = person(&[C::N2, C::Pa40, C::Rifap], 1990);
        assert!(unmet(C::N4Gp, &pa40_only).contains(&Requirement::Holds(C::N3)));
        // MF2 requires Nitrox Confirmé (new in the MFT in force).
        let mf1 = person(&[C::Mf1, C::Rifap, C::PermisCotier], 1990);
        assert!(unmet(C::Mf2, &mf1).contains(&Requirement::Holds(C::NitroxConfirme)));
        // Initiateur needs the PA20 APTITUDE (an N3 holder has it via PA40?
        // no — PA40 covers PA20 in the same family).
        let n3 = person(&[C::N3, C::Rifap], 1990);
        assert!(unmet(C::Initiateur, &n3).is_empty());
        // PESH20 opens elementary nitrox.
        let pesh = person(&[C::Pesh20], 1990);
        assert!(unmet(C::Nitrox, &pesh).is_empty());
    }

    #[test]
    fn youth_brevets_are_age_capped() {
        let adult_diver = person(&[], 1990);
        assert!(unmet(C::PlongeurBronze, &adult_diver).contains(&Requirement::MaxAge(14)));
        assert!(unmet(C::PlongeurOr, &adult_diver).contains(&Requirement::MaxAge(14)));
        let fourteen = person(&[], 2012); // exactly 14: still allowed
        assert!(!unmet(C::PlongeurBronze, &fourteen).contains(&Requirement::MaxAge(14)));
    }

    #[test]
    fn handisub_instructor_grades_are_required() {
        // EH1 needs a teaching grade (E1+): a plain N2 is refused.
        let n2 = person(&[C::N2], 1990);
        assert!(
            unmet(C::Eh1, &n2).contains(&Requirement::TeachingGrade(crate::SupervisionGrade::E1))
        );
        // An Initiateur qualifies for EH1 but not EH2 (E2 + EH1 needed).
        let initiateur = person(&[C::Initiateur, C::Rifap], 1990);
        assert!(unmet(C::Eh1, &initiateur).is_empty());
        let eh2_unmet = unmet(C::Eh2, &initiateur);
        assert!(eh2_unmet.contains(&Requirement::TeachingGrade(crate::SupervisionGrade::E2)));
        assert!(eh2_unmet.contains(&Requirement::Holds(C::Eh1)));
    }

    #[test]
    fn manual_checks_are_reported_not_silently_passed() {
        let p = person(&[C::N1], 2011); // 15-year-old
        let issues = check_eligibility(C::Pa20, &p, on());
        assert!(issues.iter().any(|i| matches!(
            i,
            EligibilityIssue::NeedsManualCheck(Requirement::DiveCount { count: 4, .. })
        )));
        assert!(issues.iter().any(|i| matches!(
            i,
            EligibilityIssue::NeedsManualCheck(Requirement::ParentalAuthorizationIfMinor)
        )));
        assert!(issues.iter().any(|i| matches!(
            i,
            EligibilityIssue::NeedsManualCheck(Requirement::MedicalCertificate)
        )));
    }

    #[test]
    fn eligible_next_replaces_fosse_preparing_options() {
        let n2_16yo = person(&[C::N1, C::N2, C::Nitrox], 2010); // 16 ans
        let next = eligible_next(&n2_16yo, on());
        assert!(next.contains(&C::Pa40));
        assert!(
            !next.contains(&C::PlongeurBronze),
            "youth brevets are capped at 14"
        );
        assert!(!next.contains(&C::Initiateur), "needs RIFAP");
        assert!(!next.contains(&C::N3), "17 minimum");
        assert!(next.contains(&C::Rifap));
        assert!(next.contains(&C::NitroxConfirme)); // 15+, N2+Nitrox held
        assert!(!next.contains(&C::N1), "already held via chain? N1 held");
    }
}
