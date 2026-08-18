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
    /// Required at DELIVERY (exam) only — one may start the cursus without
    /// it (e.g. Initiateur: RIFAP is an exam condition, MFT Dec 2025).
    /// Never blocks `eligible_next`.
    HoldsForDelivery(Certification),
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
        // RIFAP is a DELIVERY condition, not a cursus-entry one (product
        // owner confirmation 2026-08-18, consistent with the MFT pattern —
        // cf. Initiateur sheet Dec 2025).
        C::N3 => &[
            R::MinAge(17),
            R::HoldsAny(&[C::N2, C::Pa40]),
            R::HoldsForDelivery(C::Rifap),
            R::MedicalCertificate,
        ],
        // N4-GP (MFT May 2026): 17 at the exam, N3 again required (the 2023
        // PA40 bridge was removed).
        C::N4Gp => &[
            R::MinAge(17),
            R::Holds(C::N3),
            R::HoldsForDelivery(C::Rifap),
            R::SignedAptitudes("aptitudes A1-A4 signées par un E3 minimum, validité 9 mois"),
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        // N5-DPE (MFT May 2026)
        C::N5Dpe => &[
            R::MinAge(18),
            R::Holds(C::N4Gp),
            R::HoldsForDelivery(C::Rifap),
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
        // Initiateur (MFT Dec 2025): 16 at the exam, PA20 minimum; RIFAP is
        // an EXAM condition (cursus entry: 15 + PA20).
        C::Initiateur => &[
            R::MinAge(16),
            R::HoldsAptitude(DiverAptitude::Pa20),
            R::HoldsForDelivery(C::Rifap),
            R::DiveCount {
                count: 12,
                description: "plongées après l'obtention du PA20 (dispense si N3, GP-N4 ou DP-N5)",
            },
            R::ParentalAuthorizationIfMinor,
            R::MedicalCertificate,
        ],
        // MF1 (MFT Jul 2026). Entry = GP-N4; RIFAP and the permis côtier are
        // DELIVERY conditions (product rule 2026-08-18: required to be
        // certified, not to start the cursus).
        C::Mf1 => &[
            R::MinAge(18),
            R::HoldsAny(&[C::N4Gp, C::Bpjeps]),
            R::HoldsForDelivery(C::Rifap),
            R::HoldsForDelivery(C::PermisCotier),
            R::MedicalCertificate,
        ],
        // MF2 (MFT May 2026): 20 at the exam, Nitrox Confirmé now required.
        // Entry = MF1 (12 months); the rest are exam/delivery conditions.
        C::Mf2 => &[
            R::MinAge(20),
            R::Holds(C::Mf1),
            R::HoldsForDelivery(C::Rifap),
            R::HoldsForDelivery(C::NitroxConfirme),
            R::HoldsForDelivery(C::PermisCotier),
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
            R::HoldsForDelivery(C::Rifap),
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
    let implied = implied_certifications(person);
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
            // Possession checks accept IMPLIED certifications: holding a N2
            // proves the N1 (required at its delivery), even when the data
            // only records the N2.
            R::Holds(needed) => {
                if !implied.contains(needed) {
                    issues.push(EligibilityIssue::Unmet(*requirement));
                }
            }
            // Delivery-time condition: missing it never blocks starting the
            // cursus — surfaced as a manual check instead.
            R::HoldsForDelivery(needed) => {
                if !implied.contains(needed) {
                    issues.push(EligibilityIssue::NeedsManualCheck(*requirement));
                }
            }
            R::HoldsAny(options) => {
                if !options.iter().any(|o| implied.contains(o)) {
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

fn is_pesh_certification(cert: Certification) -> bool {
    matches!(
        cert,
        Certification::Pesh6
            | Certification::Pesh12
            | Certification::Pesh20
            | Certification::Pesh40
    )
}

/// Transitive closure of what holding `cert` proves. `HoldsAny` groups add
/// the INTERSECTION of their alternatives' closures: "N2 ou PA40" proves the
/// N2 either way (PA40 itself requires the N2), but "N4 ou BPJEPS" proves
/// nothing (a state-diploma path skips the federal chain).
fn certification_closure(cert: Certification, acc: &mut std::collections::BTreeSet<Certification>) {
    if !acc.insert(cert) {
        return;
    }
    for requirement in requirements(cert) {
        match requirement {
            Requirement::Holds(dep) | Requirement::HoldsForDelivery(dep) => {
                certification_closure(*dep, acc);
            }
            Requirement::HoldsAny(group) => {
                let mut common: Option<std::collections::BTreeSet<Certification>> = None;
                for alternative in *group {
                    let mut alt_closure = std::collections::BTreeSet::new();
                    certification_closure(*alternative, &mut alt_closure);
                    common = Some(match common {
                        None => alt_closure,
                        Some(c) => c.intersection(&alt_closure).copied().collect(),
                    });
                }
                for dep in common.unwrap_or_default() {
                    certification_closure(dep, acc);
                }
            }
            _ => {}
        }
    }
}

/// Certifications PROVEN by the held ones: every possession requirement of a
/// held certification was necessarily satisfied when it was delivered (a N2
/// proves the N1, a N4 proves N3→N2→N1 and the RIFAP at exam time).
pub fn implied_certifications(person: &Person) -> std::collections::BTreeSet<Certification> {
    let mut set = std::collections::BTreeSet::new();
    for cert in person.certifications {
        certification_closure(*cert, &mut set);
    }
    set
}

/// Every dive-director context, for the conferral view below.
const DP_CONTEXTS: [crate::DpContext; 5] = [
    crate::DpContext::PoolMax6m,
    crate::DpContext::ExplorationAirNitrox,
    crate::DpContext::Teaching,
    crate::DpContext::TrimixShallow,
    crate::DpContext::TrimixAll,
];

/// Prerogative view: what a set of certifications covers — aptitudes,
/// supervision AND dive-director contexts (a N5's value is DP exploration,
/// which lives outside the aptitude/supervision matrices).
struct ConferralView {
    apt: crate::Aptitudes,
    sup: crate::SupervisionCapacity,
    dp: [bool; 5],
}

/// View of `person` + an optional extra certification.
fn conferral_view(person: &Person, extra: Option<Certification>, on: NaiveDate) -> ConferralView {
    let mut certs: Vec<Certification> = person.certifications.to_vec();
    if let Some(cert) = extra {
        certs.push(cert);
    }
    let extended = Person {
        certifications: &certs,
        ..*person
    };
    ConferralView {
        apt: aptitudes(&extended, on),
        sup: crate::supervision::supervision(&extended, on),
        dp: DP_CONTEXTS.map(|ctx| crate::supervision::can_be_dp(&extended, ctx, false, on)),
    }
}

/// True when view `a` covers EVERYTHING view `b` covers.
fn view_covers(a: &ConferralView, b: &ConferralView) -> bool {
    b.apt.diver.iter().all(|x| a.apt.has(*x))
        && b.apt.gas.is_subset(&a.apt.gas)
        && (!b.sup.gp || a.sup.gp)
        && b.sup.teaching <= a.sup.teaching
        && b.sup.teaching_certified <= a.sup.teaching_certified
        && b.dp.iter().zip(a.dp.iter()).all(|(bd, ad)| !bd || *ad)
}

/// Whether `cert` alone confers ANYTHING inside our matrices (beyond the
/// universal débutant/air base). RIFAP, TIV, youth bronze/argent, EH1/EH2…
/// exist outside the aptitude space and return false.
fn confers_in_matrices(cert: Certification, person: &Person, on: NaiveDate) -> bool {
    let alone = Person {
        certifications: &[cert],
        statuses: &[],
        birthdate: person.birthdate,
        handisub: person.handisub,
    };
    let alone_apt = aptitudes(&alone, on);
    let alone_sup = crate::supervision::supervision(&alone, on);
    alone_apt
        .diver
        .iter()
        .any(|a| *a != DiverAptitude::Debutant)
        || alone_apt.gas.iter().any(|g| *g != crate::GasAptitude::Air)
        || alone_sup.gp
        || alone_sup.teaching.is_some()
}

/// True when `cert` alone would add no prerogative over what the person
/// already covers: no new diver aptitude (per `covers`), no new gas
/// aptitude, no better supervision capacity.
fn adds_no_prerogative(cert: Certification, person: &Person, on: NaiveDate) -> bool {
    if !confers_in_matrices(cert, person, on) {
        return false;
    }
    let current = conferral_view(person, None, on);
    let extended = conferral_view(person, Some(cert), on);
    view_covers(&current, &extended)
}

/// Whether `cert` alone confers ONLY diver aptitudes — the profile of a
/// brevet MODULE (PA12/PE40/PA20/PA40/PE60, N1…). Certifications carrying
/// gas or supervision value have an identity of their own (a N5 is not "a
/// worse MF1": its DP-exploration prerogative lives outside these matrices)
/// and are never absorbed.
fn confers_diver_aptitudes_only(cert: Certification, person: &Person, on: NaiveDate) -> bool {
    let alone = Person {
        certifications: &[cert],
        statuses: &[],
        birthdate: person.birthdate,
        handisub: person.handisub,
    };
    let alone_apt = aptitudes(&alone, on);
    let alone_sup = crate::supervision::supervision(&alone, on);
    alone_apt
        .diver
        .iter()
        .any(|a| *a != DiverAptitude::Debutant)
        && alone_apt.gas.iter().all(|g| *g == crate::GasAptitude::Air)
        && !alone_sup.gp
        && alone_sup.teaching.is_none()
}

/// Module absorption (product decision 2026-08-18): when a full brevet in
/// the same proposal list STRICTLY dominates a diver-aptitude module (covers
/// everything it would bring, and more), only the brevet is proposed — an N2
/// sees "N3", not "PA40 + PE60"; an N1 sees "N2", not "PA12/PE40/PA20".
/// Never absorbed: qualifications outside the matrices (RIFAP, TIV…),
/// certifications with gas or supervision value (N4, N5, Initiateur…), and
/// PESH levels (capacity-based entry choices, not a module ladder).
fn absorb_modules(
    candidates: Vec<Certification>,
    person: &Person,
    on: NaiveDate,
) -> Vec<Certification> {
    let views: Vec<(Certification, ConferralView)> = candidates
        .iter()
        .map(|c| (*c, conferral_view(person, Some(*c), on)))
        .collect();
    candidates
        .iter()
        .filter(|cert| {
            if !confers_diver_aptitudes_only(**cert, person, on) || is_pesh_certification(**cert) {
                return true;
            }
            let mine = views.iter().find(|(c, _)| c == *cert).map(|(_, v)| v);
            let Some(mine) = mine else { return true };
            !views.iter().any(|(other, view)| {
                other != *cert && view_covers(view, mine) && !view_covers(mine, view)
            })
        })
        .copied()
        .collect()
}

/// A certification is REDUNDANT for a person when it would confer nothing
/// they do not already cover AND unlocks no further certification. A N2
/// makes PE12/PA12/PE40/PA20 pointless (and proves N1); a MF1 makes
/// Initiateur pointless. But a Plongeur d'Or still needs the N1 — same
/// aptitude space, yet the N1 is the required step towards the N2 (chain
/// value). Qualifications outside the aptitude matrices (RIFAP, TIV, youth
/// bronze/argent, EH1/EH2…) are never redundant.
pub fn is_redundant(cert: Certification, person: &Person, on: NaiveDate) -> bool {
    let implied = implied_certifications(person);
    if implied.contains(&cert) {
        return true;
    }
    if !adds_no_prerogative(cert, person, on) {
        return false;
    }
    // Adds no prerogative today — still worth it if some reachable, useful
    // certification strictly requires it (one level deep).
    let has_chain_value = Certification::ALL.into_iter().any(|target| {
        if implied.contains(&target)
            || target.is_externally_delivered()
            || adds_no_prerogative(target, person, on)
        {
            return false;
        }
        requirements(target).iter().any(|r| match r {
            Requirement::Holds(dep) | Requirement::HoldsForDelivery(dep) => *dep == cert,
            Requirement::HoldsAny(group) => {
                group.contains(&cert) && !group.iter().any(|alt| implied.contains(alt))
            }
            _ => false,
        })
    });
    !has_chain_value
}

/// Everything holding `cert` makes ACQUIRED: proven certifications (its
/// requirement chain) plus those whose prerogatives it fully covers —
/// person-independent (adult reference; handisub on so the PESH ladder is
/// covered too). Powers UI trees where checking "N2" ticks
/// N1/PE12/PA12/PE40/PA20 automatically.
pub fn covered_by(cert: Certification) -> Vec<Certification> {
    let reference = Person {
        certifications: &[cert],
        statuses: &[],
        // Adult reference: coverage is about the certification lattice, not
        // a specific person (age-of-exercise gates do not apply here).
        birthdate: NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"),
        handisub: true,
    };
    let on = reference.birthdate + chrono::Days::new(365 * 40);
    let implied = implied_certifications(&reference);
    Certification::ALL
        .into_iter()
        .filter(|other| *other != cert)
        .filter(|other| {
            implied.contains(other) || {
                // Covered: adding `other` would bring nothing.
                confers_in_matrices(*other, &reference, on)
                    && adds_no_prerogative(*other, &reference, on)
            }
        })
        .collect()
}

/// Certifications the person could start preparing now: not held, not
/// redundant, PESH only inside the Handisub pathway, and no machine-checkable
/// requirement unmet (manual checks remain to confirm).
pub fn eligible_next(person: &Person, on: NaiveDate) -> Vec<Certification> {
    let implied = implied_certifications(person);
    let candidates: Vec<Certification> = Certification::ALL
        .into_iter()
        .filter(|cert| !cert.is_externally_delivered())
        .filter(|cert| !implied.contains(cert))
        .filter(|cert| person.practices_handisub() || !is_pesh_certification(*cert))
        .filter(|cert| !is_redundant(*cert, person, on))
        .filter(|cert| {
            check_eligibility(*cert, person, on)
                .iter()
                .all(|issue| matches!(issue, EligibilityIssue::NeedsManualCheck(_)))
        })
        .collect();
    absorb_modules(candidates, person, on)
}

/// Certifications WORTH preparing: same exclusions as [`eligible_next`]
/// (held, redundant, PESH gate, external) plus permanently-impossible ones
/// (MaxAge exceeded) — but unmet ENTRY requirements do not exclude: the club
/// decides when a cursus starts (e.g. N3 shows for a N2 without PA40 yet).
pub fn preparable_certifications(person: &Person, on: NaiveDate) -> Vec<Certification> {
    let implied = implied_certifications(person);
    Certification::ALL
        .into_iter()
        .filter(|cert| !cert.is_externally_delivered())
        .filter(|cert| !implied.contains(cert))
        .filter(|cert| person.practices_handisub() || !is_pesh_certification(*cert))
        .filter(|cert| !is_redundant(*cert, person, on))
        .filter(|cert| {
            requirements(*cert).iter().all(|r| match r {
                Requirement::MaxAge(age) => person.age_on(on) <= *age,
                _ => true,
            })
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
            handisub: false,
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
        // RIFAP is a delivery condition: reported as a manual check, never
        // as a blocking unmet entry requirement.
        assert!(!unmet(C::N3, &nobody).contains(&Requirement::Holds(C::Rifap)));
        assert!(check_eligibility(C::N3, &nobody, on())
            .iter()
            .any(|i| matches!(
                i,
                EligibilityIssue::NeedsManualCheck(Requirement::HoldsForDelivery(C::Rifap))
            )));
        // N4 requires N3 again (the 2023 PA40 bridge was removed).
        let pa40_only = person(&[C::N2, C::Pa40, C::Rifap], 1990);
        assert!(unmet(C::N4Gp, &pa40_only).contains(&Requirement::Holds(C::N3)));
        // MF2 requires Nitrox Confirmé (new in the MFT in force) — at
        // DELIVERY: reported as a manual check, not a blocking entry one.
        let mf1 = person(&[C::Mf1, C::Rifap, C::PermisCotier], 1990);
        assert!(!unmet(C::Mf2, &mf1).contains(&Requirement::Holds(C::NitroxConfirme)));
        assert!(check_eligibility(C::Mf2, &mf1, on())
            .iter()
            .any(|i| matches!(
                i,
                EligibilityIssue::NeedsManualCheck(Requirement::HoldsForDelivery(
                    C::NitroxConfirme
                ))
            )));
        assert!(eligible_next(&mf1, on()).contains(&C::Mf2));
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
        assert!(
            next.contains(&C::Initiateur),
            "RIFAP is an exam condition, not an entry one (MFT Dec 2025)"
        );
        assert!(!next.contains(&C::N3), "17 minimum");
        assert!(next.contains(&C::Rifap));
        assert!(next.contains(&C::NitroxConfirme)); // 15+, N2+Nitrox held
        assert!(!next.contains(&C::N1), "N1 held");
    }

    #[test]
    fn redundant_certifications_are_not_proposed() {
        let adult_n2 = person(&[C::N2], 1990);
        let next = eligible_next(&adult_n2, on());
        // Everything the N2 already covers (or proves) disappears.
        for covered in [C::Pe12, C::Pa12, C::Pe40, C::Pa20, C::N1] {
            assert!(!next.contains(&covered), "{covered:?} is covered by N2");
            assert!(is_redundant(covered, &adult_n2, on()));
        }
        // Module absorption: the N3 (reachable — RIFAP is a delivery
        // condition) dominates its PA40/PE60 modules.
        assert!(next.contains(&C::N3), "{next:?}");
        assert!(!next.contains(&C::Pa40), "absorbed by N3: {next:?}");
        assert!(!next.contains(&C::Pe60), "absorbed by N3: {next:?}");
        // Qualifications outside the matrices are never absorbed.
        assert!(next.contains(&C::Rifap));
        // Implied N1 satisfies the Nitrox prerequisite even without the row.
        assert!(next.contains(&C::Nitrox));

        // An adult N1 sees the N2, not its PA12/PE40/PA20 modules…
        let adult_n1 = person(&[C::N1], 1990);
        let next = eligible_next(&adult_n1, on());
        assert!(next.contains(&C::N2), "{next:?}");
        for module in [C::Pa12, C::Pe40, C::Pa20] {
            assert!(!next.contains(&module), "absorbed by N2: {next:?}");
        }
        // …but a 14-year-old N1 sees PE40 (N2 opens at 15 only).
        let young_n1 = person(&[C::N1], 2012);
        let next = eligible_next(&young_n1, on());
        assert!(!next.contains(&C::N2), "15 minimum: {next:?}");
        assert!(next.contains(&C::Pe40), "{next:?}");

        // A MF1 (E3) makes Initiateur (E1) pointless.
        let mf1 = person(&[C::N3, C::N4Gp, C::Mf1], 1985);
        assert!(is_redundant(C::Initiateur, &mf1, on()));
        assert!(!eligible_next(&mf1, on()).contains(&C::Initiateur));

        // But a Plongeur d'Or still needs the N1 (chain to N2) despite the
        // identical 0-20 m aptitude space.
        let gold_12yo = person(&[C::PlongeurOr], 2014);
        assert!(!is_redundant(C::N1, &gold_12yo, on()));
        assert!(eligible_next(&gold_12yo, on()).contains(&C::N1));
    }

    #[test]
    fn n4_gp_is_offered_the_instructor_and_dp_pathways() {
        // Bare N4 row in the database: N3/N2/N1/RIFAP are implied.
        let n4 = person(&[C::N4Gp], 1990);
        let next = eligible_next(&n4, on());
        // Both distinct pathways stay visible: MF1 (E3, teaching) must not
        // absorb N5 (club DP without teaching) nor Initiateur (E2 with GP).
        assert!(next.contains(&C::Mf1), "{next:?}");
        assert!(next.contains(&C::N5Dpe), "{next:?}");
        assert!(next.contains(&C::Initiateur), "{next:?}");
        // Implied by the N4 chain → never proposed.
        for implied in [C::N3, C::N2, C::N1, C::Rifap] {
            assert!(!next.contains(&implied), "{implied:?} implied: {next:?}");
        }
        // Trimix élémentaire still gated by the (entry) Nitrox Confirmé.
        assert!(!next.contains(&C::TrimixElementaire), "{next:?}");
        // Base nitrox opens (implied N1 through the N4→N3→"N2 ou PA40"
        // chain — both alternatives prove the N2, hence the N1).
        assert!(next.contains(&C::Nitrox), "{next:?}");
    }

    #[test]
    fn pesh_certifications_are_gated_to_the_handisub_pathway() {
        let adult_n1 = person(&[C::N1], 1990);
        let next = eligible_next(&adult_n1, on());
        assert!(!next.contains(&C::Pesh6), "PESH require the handisub flag");

        let handisub = Person {
            handisub: true,
            ..person(&[C::N1], 1990)
        };
        assert!(eligible_next(&handisub, on()).contains(&C::Pesh6));

        // Holding a PESH already puts you in the pathway (no flag needed).
        let pesh20 = person(&[C::Pesh20], 1990);
        assert!(eligible_next(&pesh20, on()).contains(&C::Pesh40));
    }

    #[test]
    fn covered_by_matches_the_lattice() {
        // N2: proves the N1 and covers every module up to PE40/PA20 — but
        // nothing from the N3 zone, and never RIFAP (outside the matrices).
        let n2 = covered_by(C::N2);
        for cert in [C::N1, C::Pe12, C::Pa12, C::Pe40, C::Pa20] {
            assert!(n2.contains(&cert), "N2 should cover {cert:?}: {n2:?}");
        }
        for cert in [C::Pa40, C::Pe60, C::Rifap] {
            assert!(!n2.contains(&cert), "N2 must not cover {cert:?}: {n2:?}");
        }
        // MF1: proves its delivery chain (N4→N3, RIFAP, permis côtier) and
        // covers the Initiateur (E1 < E3).
        let mf1 = covered_by(C::Mf1);
        for cert in [C::N4Gp, C::N3, C::Initiateur, C::Rifap, C::PermisCotier] {
            assert!(mf1.contains(&cert), "MF1 should cover {cert:?}: {mf1:?}");
        }
        // RIFAP lives outside the aptitude matrices: it covers nothing.
        assert!(covered_by(C::Rifap).is_empty());
        // PESH40 covers the whole PESH ladder below it.
        let pesh40 = covered_by(C::Pesh40);
        for cert in [C::Pesh20, C::Pesh12, C::Pesh6] {
            assert!(
                pesh40.contains(&cert),
                "PESH40 should cover {cert:?}: {pesh40:?}"
            );
        }
    }

    #[test]
    fn preparable_tolerates_unmet_entry_requirements() {
        let adult_n2 = person(&[C::N2, C::Rifap], 1990);
        let preparable = preparable_certifications(&adult_n2, on());
        // N3 shows even though the PA40 module is not acquired yet.
        assert!(preparable.contains(&C::N3));
        assert!(preparable.contains(&C::Initiateur));
        // Redundant / impossible ones still excluded.
        assert!(!preparable.contains(&C::Pe40));
        assert!(!preparable.contains(&C::PlongeurBronze), "MaxAge 14");
        assert!(!preparable.contains(&C::Pesh6), "handisub gate");
        // Externally delivered titles are not club-preparable.
        assert!(!preparable.contains(&C::Bees1));
    }
}
