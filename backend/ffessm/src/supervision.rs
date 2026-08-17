//! Matrix 3: supervision capacity (annexe III-15b, 2026-01-17 + MFT sheets)
//! and dive-director rules (annexe III-15a + art. A322-72/98/99).

use chrono::NaiveDate;

use crate::{Certification, Person, SupervisionGrade, TrainingStatus};

/// What a person may supervise. `gp` = palanquée guide in EXPLORATION
/// (annexe III-16b); `teaching` = teaching grade E1-E4 (annexe III-16a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SupervisionCapacity {
    pub gp: bool,
    /// Best teaching grade, INCLUDING dated statuses (stagiaire MF1 = E2).
    pub teaching: Option<SupervisionGrade>,
    /// Best teaching grade from CERTIFICATIONS only. The stagiaire
    /// pédagogique MF1 teaches but can never be DP (MFT MF1 Jul 2026), so
    /// DP rules look at this field.
    pub teaching_certified: Option<SupervisionGrade>,
}

/// Supervision capacity exercisable on `on`.
///
/// Age gate: the MFT (GP-N4, May 2026) suspends the exercise of ANY
/// palanquée supervision before 18 ("dans l'attente de la décision de la
/// Direction des Sports") — transitional, see analysis §8. Teaching grades
/// all imply >= 18 in practice except a 16-17-year-old Initiateur, whom the
/// MFT does allow to teach in the 0-6 m space — so the 18 gate applies to
/// `gp` only.
pub fn supervision(person: &Person, on: NaiveDate) -> SupervisionCapacity {
    use Certification as C;
    let age = person.age_on(on);

    // GP flag: N4/GP or a certification whose chain includes it.
    // Source: annexe III-15b row "GP" + MFT prerequisite chains.
    let gp_certs = [
        C::N4Gp,
        C::N5Dpe,
        C::Mf1,
        C::Mf2,
        C::Beppa,
        C::Bpjeps,
        C::Bees1,
        C::Bees2,
        C::Dejeps,
        C::DejepsPlongeeProfonde,
        C::Desjeps,
    ];
    let gp = person.holds_any(&gp_certs) && age >= 18;

    // Teaching grade from certifications (annexe III-15b).
    let mut certified: Option<SupervisionGrade> = None;
    let upgrade = |current: &mut Option<SupervisionGrade>, grade: SupervisionGrade| {
        if current.is_none_or(|c| grade > c) {
            *current = Some(grade);
        }
    };

    if person.holds(C::Initiateur) {
        upgrade(&mut certified, SupervisionGrade::E1);
        // Initiateur + GP-N4 = E2 (MFT Initiateur Dec 2025, annexe III-15b).
        if person.holds(C::N4Gp) {
            upgrade(&mut certified, SupervisionGrade::E2);
        }
    }
    // RESERVE: annexe III-15b lists BPJEPS across the GP/E1/E2 rows
    // (historically depends on its optional units); we retain E2, the
    // established reading. See analysis §8.
    if person.holds(C::Bpjeps) {
        upgrade(&mut certified, SupervisionGrade::E2);
    }
    if person.holds_any(&[C::Mf1, C::Bees1, C::Dejeps]) {
        upgrade(&mut certified, SupervisionGrade::E3);
    }
    if person.holds_any(&[
        C::Mf2,
        C::Beppa,
        C::Bees2,
        C::DejepsPlongeeProfonde,
        C::Desjeps,
    ]) {
        upgrade(&mut certified, SupervisionGrade::E4);
    }

    // Stagiaire pédagogique MF1 = E2 while the livret is valid, teaching
    // only — never DP (annexe III-15b + MFT MF1 Jul 2026).
    let mut teaching = certified;
    for status in person.statuses {
        let TrainingStatus::StagiairePedagogiqueMf1 { livret_valid_until } = status;
        if *livret_valid_until >= on {
            upgrade(&mut teaching, SupervisionGrade::E2);
        }
    }

    SupervisionCapacity {
        gp,
        teaching,
        teaching_certified: certified,
    }
}

/// Dive-director contexts (annexe III-15a, 2026-01-17 + art. A322-98).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpContext {
    /// Swimming pool or pit <= 6 m (art. A322-98): E1 minimum, 18+.
    PoolMax6m,
    /// Exploration dives on air/nitrox in open water: DPE/N5 or E3+.
    ExplorationAirNitrox,
    /// Teaching dives on air/nitrox: E3 minimum.
    Teaching,
    /// Trimix/heliox: teaching <= 40 m, exploration <= 70 m: E3 minimum.
    TrimixShallow,
    /// Trimix/heliox beyond: E4 minimum (MF2/BEPPA/BEES2/…).
    TrimixAll,
}

/// Whether `person` may act as Directeur de Plongée in `ctx`.
/// `uses_mixtures`: for nitrox/trimix dives the DP must additionally hold
/// PN-C or PTH-120 (art. A322-72 / annexe III-15a).
pub fn can_be_dp(person: &Person, ctx: DpContext, uses_mixtures: bool, on: NaiveDate) -> bool {
    use Certification as C;
    let cap = supervision(person, on);
    // DP rules ignore status-derived grades (stagiaire MF1 is never DP).
    let teaching_at_least =
        |grade: SupervisionGrade| cap.teaching_certified.is_some_and(|t| t >= grade);

    let base = match ctx {
        DpContext::PoolMax6m => teaching_at_least(SupervisionGrade::E1) && person.age_on(on) >= 18,
        DpContext::ExplorationAirNitrox => {
            (person.holds(C::N5Dpe) && person.age_on(on) >= 18)
                || teaching_at_least(SupervisionGrade::E3)
        }
        DpContext::Teaching | DpContext::TrimixShallow => teaching_at_least(SupervisionGrade::E3),
        DpContext::TrimixAll => teaching_at_least(SupervisionGrade::E4),
    };
    if !base {
        return false;
    }
    if uses_mixtures {
        return person.holds_any(&[C::NitroxConfirme, C::Trimix]);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Certification as C;
    use SupervisionGrade as G;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn on() -> NaiveDate {
        d(2026, 8, 17)
    }
    fn adult(certs: &'static [C]) -> Person<'static> {
        Person {
            certifications: certs,
            statuses: &[],
            birthdate: d(1990, 1, 1),
        }
    }

    #[test]
    fn grade_matrix_matches_annexe_iii_15b() {
        let cases: &[(&[C], bool, Option<G>)] = &[
            (&[C::N3], false, None),
            (&[C::N4Gp], true, None),
            (&[C::Initiateur], false, Some(G::E1)),
            (&[C::Initiateur, C::N4Gp], true, Some(G::E2)),
            (&[C::Mf1], true, Some(G::E3)),
            (&[C::Bees1], true, Some(G::E3)),
            (&[C::Dejeps], true, Some(G::E3)),
            (&[C::Mf2], true, Some(G::E4)),
            (&[C::Beppa], true, Some(G::E4)),
            (&[C::DejepsPlongeeProfonde], true, Some(G::E4)),
            (&[C::Desjeps], true, Some(G::E4)),
            (&[C::Bpjeps], true, Some(G::E2)), // RESERVE, see analysis
        ];
        for (certs, gp, teaching) in cases {
            let cap = supervision(&adult(certs), on());
            assert_eq!(cap.gp, *gp, "{certs:?}");
            assert_eq!(cap.teaching, *teaching, "{certs:?}");
        }
    }

    #[test]
    fn gp_exercise_is_gated_at_18_but_initiateur_teaching_is_not() {
        // 17-year-old holding N4 (exam possible at 17, MFT May 2026):
        // no supervision exercise before 18 (transitional gate).
        let minor_n4 = Person {
            certifications: &[C::N4Gp],
            statuses: &[],
            birthdate: d(2009, 1, 1), // 17 in Aug 2026
        };
        assert!(!supervision(&minor_n4, on()).gp);
        // Same person the day of their 18th birthday.
        assert!(supervision(&minor_n4, d(2027, 1, 1)).gp);

        // 16-year-old Initiateur teaches in the 0-6 m space (E1)…
        let young_initiateur = Person {
            certifications: &[C::Initiateur],
            statuses: &[],
            birthdate: d(2010, 1, 1), // 16
        };
        assert_eq!(supervision(&young_initiateur, on()).teaching, Some(G::E1));
        // …but cannot be pool DP before 18 (A322-98 + MFT).
        assert!(!can_be_dp(
            &young_initiateur,
            DpContext::PoolMax6m,
            false,
            on()
        ));
    }

    #[test]
    fn stagiaire_mf1_is_e2_only_while_livret_is_valid() {
        let statuses = [TrainingStatus::StagiairePedagogiqueMf1 {
            livret_valid_until: d(2026, 12, 31),
        }];
        let p = Person {
            certifications: &[C::N4Gp, C::Initiateur],
            statuses: &statuses,
            birthdate: d(1990, 1, 1),
        };
        assert_eq!(supervision(&p, on()).teaching, Some(G::E2));
        // After expiry, falls back to whatever the certifications give (E2
        // here because Initiateur+N4).
        assert_eq!(supervision(&p, d(2027, 6, 1)).teaching, Some(G::E2));

        let solo = Person {
            certifications: &[C::N4Gp],
            statuses: &statuses,
            birthdate: d(1990, 1, 1),
        };
        assert_eq!(supervision(&solo, on()).teaching, Some(G::E2));
        assert_eq!(supervision(&solo, d(2027, 6, 1)).teaching, None);
    }

    #[test]
    fn dp_matrix_matches_annexe_iii_15a() {
        let n5 = adult(&[C::N5Dpe]);
        let gp = adult(&[C::N4Gp]);
        let e1 = adult(&[C::Initiateur]);
        let e3 = adult(&[C::Mf1]);
        let e4 = adult(&[C::Mf2]);

        // Pool <= 6 m: E1 minimum.
        assert!(can_be_dp(&e1, DpContext::PoolMax6m, false, on()));
        assert!(!can_be_dp(&gp, DpContext::PoolMax6m, false, on()));
        // Exploration air/nitrox: N5/DPE or E3+ — a plain GP is NOT enough.
        assert!(can_be_dp(&n5, DpContext::ExplorationAirNitrox, false, on()));
        assert!(can_be_dp(&e3, DpContext::ExplorationAirNitrox, false, on()));
        assert!(!can_be_dp(
            &gp,
            DpContext::ExplorationAirNitrox,
            false,
            on()
        ));
        assert!(!can_be_dp(
            &e1,
            DpContext::ExplorationAirNitrox,
            false,
            on()
        ));
        // Teaching: E3 minimum — an N5 is NOT enough.
        assert!(can_be_dp(&e3, DpContext::Teaching, false, on()));
        assert!(!can_be_dp(&n5, DpContext::Teaching, false, on()));
        // Deep trimix: E4 only.
        assert!(can_be_dp(&e4, DpContext::TrimixAll, false, on()));
        assert!(!can_be_dp(&e3, DpContext::TrimixAll, false, on()));
        // Mixtures require PN-C or PTH-120 on top.
        assert!(!can_be_dp(&e3, DpContext::ExplorationAirNitrox, true, on()));
        let e3_pnc = adult(&[C::Mf1, C::NitroxConfirme]);
        assert!(can_be_dp(
            &e3_pnc,
            DpContext::ExplorationAirNitrox,
            true,
            on()
        ));
    }
}
