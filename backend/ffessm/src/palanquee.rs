//! Matrix 5: palanquée composition (Code du Sport annexes III-16a/III-16b,
//! verified against the official MFT extract of June 2026).

use chrono::NaiveDate;

use crate::derive::exercisable_aptitudes;
use crate::supervision::supervision;
use crate::{DiverAptitude, ExerciseRestriction, Person, SupervisionGrade};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PalanqueeKind {
    /// First scuba dive: exactly one student (annexe III-16a).
    Bapteme,
    Exploration,
    Teaching,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PalanqueeViolation {
    /// Air/nitrox dives are capped at 60 m (art. A322-76).
    DepthExceedsAirNitroxLimit { depth_m: u8 },
    /// This member has no exercisable aptitude covering the planned depth.
    MemberLacksAptitude { member_index: usize, depth_m: u8 },
    /// Guided palanquée exceeds 4 members (the +1 slot needs a GP/P4).
    TooManyGuidedMembers { count: usize },
    /// 0-60 m guided space has no +1 slot (annexe III-16b).
    NoExtraSlotBeyond40m { count: usize },
    /// Autonomous palanquée must have 2 or 3 members.
    AutonomousGroupSize { count: usize },
    /// A baptême takes exactly one student.
    BaptemeSingleStudent { count: usize },
    /// Teaching and baptêmes always need a qualified instructor.
    GuideRequired,
    /// The guide's grade does not cover this depth/kind.
    GuideNotQualified {
        required_teaching_grade: Option<SupervisionGrade>,
        gp_sufficient: bool,
    },
}

impl PalanqueeViolation {
    pub fn label_fr(&self) -> String {
        match self {
            PalanqueeViolation::DepthExceedsAirNitroxLimit { depth_m } => {
                format!("Profondeur {depth_m} m : l'air et le nitrox sont limités à 60 m")
            }
            PalanqueeViolation::MemberLacksAptitude {
                member_index,
                depth_m,
            } => format!(
                "Le plongeur n°{} n'a pas d'aptitude exerçable couvrant {depth_m} m",
                member_index + 1
            ),
            PalanqueeViolation::TooManyGuidedMembers { count } => format!(
                "{count} plongeurs encadrés : maximum 4 (5e possible uniquement s'il est GP/P4)"
            ),
            PalanqueeViolation::NoExtraSlotBeyond40m { count } => format!(
                "{count} plongeurs : au-delà de 40 m, 4 plongeurs encadrés maximum, sans 5e"
            ),
            PalanqueeViolation::AutonomousGroupSize { count } => {
                format!("Palanquée autonome de {count} : il faut 2 ou 3 plongeurs")
            }
            PalanqueeViolation::BaptemeSingleStudent { count } => {
                format!("Baptême avec {count} élèves : un seul élève par baptême")
            }
            PalanqueeViolation::GuideRequired => {
                "Un encadrant qualifié est requis pour cette palanquée".to_string()
            }
            PalanqueeViolation::GuideNotQualified {
                required_teaching_grade,
                gp_sufficient,
            } => match (required_teaching_grade, gp_sufficient) {
                (Some(grade), true) => {
                    format!("Encadrant insuffisant : il faut au minimum {grade} ou un GP/P4")
                }
                (Some(grade), false) => {
                    format!("Encadrant insuffisant : il faut au minimum {grade}")
                }
                (None, _) => "Encadrant non qualifié".to_string(),
            },
        }
    }
}

/// Non-blocking findings (product decision 2026-08-17: Handisub clinical
/// constraints are surfaced as warnings, not violations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PalanqueeWarning {
    /// A PESH diver is present: the guide must hold the appropriate Handisub
    /// qualification (EH1 for moderate, EH2 for major handicap — severity is
    /// not tracked by this library).
    HandisubQualificationCheck { member_index: usize },
    /// PESH40 clinical dive profile applies.
    Pesh40ClinicalProfile,
    /// Minors diving in autonomy: informed parental authorization + the
    /// palanquée must be informed (MFT N2/N3).
    MinorAutonomyFormalities { member_index: usize },
    /// Exercise restriction attached to a member (e.g. PE40 under 16).
    MemberRestriction {
        member_index: usize,
        restriction: ExerciseRestriction,
    },
    /// Teaching dive beyond the member's held aptitude — legal (that is what
    /// formation is, A322-83..87), surfaced for awareness.
    TrainingBeyondHeldAptitude { member_index: usize },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PalanqueeReport {
    pub violations: Vec<PalanqueeViolation>,
    pub warnings: Vec<PalanqueeWarning>,
}

impl PalanqueeReport {
    pub fn is_legal(&self) -> bool {
        self.violations.is_empty()
    }
}

fn required_grade_for_depth(depth_m: u8) -> SupervisionGrade {
    // Annexes III-16a/III-16b: E1 0-6, E2 0-20 (and 0-12), E3 0-40, E4 0-60.
    match depth_m {
        0..=6 => SupervisionGrade::E1,
        7..=20 => SupervisionGrade::E2,
        21..=40 => SupervisionGrade::E3,
        _ => SupervisionGrade::E4,
    }
}

/// Validates a palanquée against the Code du Sport tables.
///
/// `guide: None` means an AUTONOMOUS palanquée (every member must hold an
/// exercisable PA aptitude covering the depth). The DP presence/organisation
/// is the caller's concern (note: without DP, PA-60 groups are limited to
/// 0-40 m — art. A322-99).
pub fn validate_palanquee(
    kind: PalanqueeKind,
    depth_m: u8,
    guide: Option<&Person>,
    members: &[&Person],
    on: NaiveDate,
) -> PalanqueeReport {
    let mut report = PalanqueeReport::default();

    if depth_m > 60 {
        report
            .violations
            .push(PalanqueeViolation::DepthExceedsAirNitroxLimit { depth_m });
        return report;
    }

    let member_states: Vec<_> = members
        .iter()
        .map(|p| exercisable_aptitudes(p, on))
        .collect();

    match guide {
        None => validate_autonomous(depth_m, members, &member_states, &mut report),
        Some(guide) => validate_guided(
            kind,
            depth_m,
            guide,
            members,
            &member_states,
            on,
            &mut report,
        ),
    }

    // Per-member warnings (both modes).
    for (index, (aptitudes, restrictions)) in member_states.iter().enumerate() {
        // Any PESH diver present ⇒ the guide needs the matching Handisub
        // qualification (EH1/EH2 by handicap severity, or GH) — severity is
        // not tracked here, so this is a warning by product decision.
        if aptitudes.diver.iter().any(|a| a.is_pesh()) {
            report
                .warnings
                .push(PalanqueeWarning::HandisubQualificationCheck {
                    member_index: index,
                });
        }
        for restriction in restrictions {
            match restriction {
                ExerciseRestriction::Pesh40ClinicalProfile => {
                    if !report
                        .warnings
                        .contains(&PalanqueeWarning::Pesh40ClinicalProfile)
                    {
                        report
                            .warnings
                            .push(PalanqueeWarning::Pesh40ClinicalProfile);
                    }
                }
                ExerciseRestriction::MinorAutonomyFormalities => {
                    if guide.is_none() {
                        report
                            .warnings
                            .push(PalanqueeWarning::MinorAutonomyFormalities {
                                member_index: index,
                            });
                    }
                }
                // Autonomy-specific rule: noise on guided dives.
                ExerciseRestriction::MinorAutonomyAirNitroxOnly => {
                    if guide.is_none() {
                        report.warnings.push(PalanqueeWarning::MemberRestriction {
                            member_index: index,
                            restriction: *restriction,
                        });
                    }
                }
                other => report.warnings.push(PalanqueeWarning::MemberRestriction {
                    member_index: index,
                    restriction: *other,
                }),
            }
        }
    }
    report
}

fn validate_autonomous(
    depth_m: u8,
    members: &[&Person],
    states: &[(crate::derive::Aptitudes, Vec<ExerciseRestriction>)],
    report: &mut PalanqueeReport,
) {
    // Annexe III-16b: autonomous palanquées have 2 to 3 divers.
    if !(2..=3).contains(&members.len()) {
        report
            .violations
            .push(PalanqueeViolation::AutonomousGroupSize {
                count: members.len(),
            });
    }
    for (index, (aptitudes, _)) in states.iter().enumerate() {
        let ok = aptitudes.max_autonomous_depth_m().unwrap_or(0) >= depth_m;
        if !ok {
            report
                .violations
                .push(PalanqueeViolation::MemberLacksAptitude {
                    member_index: index,
                    depth_m,
                });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_guided(
    kind: PalanqueeKind,
    depth_m: u8,
    guide: &Person,
    members: &[&Person],
    states: &[(crate::derive::Aptitudes, Vec<ExerciseRestriction>)],
    on: NaiveDate,
    report: &mut PalanqueeReport,
) {
    // Effectifs (annexes III-16a/16b): baptême 1; otherwise 4, +1 if the
    // extra member is themselves GP/P4 — except in the 0-60 space.
    match kind {
        PalanqueeKind::Bapteme => {
            if members.len() != 1 {
                report
                    .violations
                    .push(PalanqueeViolation::BaptemeSingleStudent {
                        count: members.len(),
                    });
            }
        }
        PalanqueeKind::Exploration | PalanqueeKind::Teaching => {
            if members.len() > 5 {
                report
                    .violations
                    .push(PalanqueeViolation::TooManyGuidedMembers {
                        count: members.len(),
                    });
            } else if members.len() == 5 {
                if depth_m > 40 {
                    report
                        .violations
                        .push(PalanqueeViolation::NoExtraSlotBeyond40m {
                            count: members.len(),
                        });
                } else {
                    let extra_is_gp = members.iter().any(|m| supervision(m, on).gp);
                    if !extra_is_gp {
                        report
                            .violations
                            .push(PalanqueeViolation::TooManyGuidedMembers {
                                count: members.len(),
                            });
                    }
                }
            }
        }
    }

    // Guide qualification.
    let capacity = supervision(guide, on);
    let required = required_grade_for_depth(depth_m);
    let teaching_ok = capacity.teaching.is_some_and(|t| t >= required);
    let qualified = match kind {
        // Exploration (III-16b): GP/P4 suffices up to 40 m; 0-60 m needs E4.
        PalanqueeKind::Exploration => {
            if depth_m > 40 {
                teaching_ok
            } else {
                capacity.gp || teaching_ok
            }
        }
        // Teaching & baptême (III-16a): teaching grade only.
        PalanqueeKind::Teaching | PalanqueeKind::Bapteme => teaching_ok,
    };
    if !qualified {
        report
            .violations
            .push(PalanqueeViolation::GuideNotQualified {
                required_teaching_grade: Some(required),
                gp_sufficient: matches!(kind, PalanqueeKind::Exploration) && depth_m <= 40,
            });
    }

    // Members. In EXPLORATION (and baptêmes), divers must hold a guided
    // (PE/débutant) or PESH aptitude covering the depth. In TEACHING, the
    // space is bounded by the instructor's grade — students dive beyond
    // their held aptitude by definition of a formation (art. A322-83..87,
    // annexe III-16a) — surfaced as a warning, never a violation.
    for (index, (aptitudes, _)) in states.iter().enumerate() {
        let guided_depth = aptitudes.max_guided_depth_m().unwrap_or(0);
        let pesh_depth = aptitudes
            .diver
            .iter()
            .filter(|a| a.is_pesh())
            .map(DiverAptitude::max_depth_m)
            .max()
            .unwrap_or(0);
        if guided_depth.max(pesh_depth) < depth_m {
            match kind {
                PalanqueeKind::Exploration | PalanqueeKind::Bapteme => {
                    report
                        .violations
                        .push(PalanqueeViolation::MemberLacksAptitude {
                            member_index: index,
                            depth_m,
                        });
                }
                PalanqueeKind::Teaching => {
                    report
                        .warnings
                        .push(PalanqueeWarning::TrainingBeyondHeldAptitude {
                            member_index: index,
                        });
                }
            }
        }
    }
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
    fn adult(certs: &'static [C]) -> Person<'static> {
        Person {
            certifications: certs,
            statuses: &[],
            birthdate: d(1990, 1, 1),
        }
    }

    const N1: &[C] = &[C::N1];
    const N2: &[C] = &[C::N2];
    const N3: &[C] = &[C::N3];
    const GP: &[C] = &[C::N4Gp];
    const E4: &[C] = &[C::Mf2];
    const NOBODY: &[C] = &[];

    #[test]
    fn exploration_table_iii_16b() {
        let gp = adult(GP);
        let e4 = adult(E4);

        // GP guiding 4 PE-20 divers at 20 m: legal.
        let m = [&adult(N1), &adult(N1), &adult(N1), &adult(N1)];
        let r = validate_palanquee(PalanqueeKind::Exploration, 20, Some(&gp), &m, on());
        assert!(r.is_legal(), "{:?}", r.violations);

        // Same at 40 m: N1 (PE20) members lack the aptitude.
        let r = validate_palanquee(PalanqueeKind::Exploration, 40, Some(&gp), &m, on());
        assert_eq!(
            r.violations.len(),
            4,
            "each PE20 member flagged at 40 m: {:?}",
            r.violations
        );

        // GP guiding PE-40 divers (N2) at 40 m: legal.
        let m = [&adult(N2), &adult(N2)];
        let r = validate_palanquee(PalanqueeKind::Exploration, 40, Some(&gp), &m, on());
        assert!(r.is_legal(), "{:?}", r.violations);

        // 0-60 m guided: a GP is NOT enough, E4 required; members need PE60.
        let m = [&adult(N3)];
        let r = validate_palanquee(PalanqueeKind::Exploration, 60, Some(&gp), &m, on());
        assert!(r
            .violations
            .iter()
            .any(|v| matches!(v, PalanqueeViolation::GuideNotQualified { .. })));
        let r = validate_palanquee(PalanqueeKind::Exploration, 60, Some(&e4), &m, on());
        assert!(r.is_legal(), "{:?}", r.violations);
    }

    #[test]
    fn the_fifth_guided_member_must_be_a_gp() {
        let guide = adult(GP);
        let plain = adult(N1);
        let extra_gp = adult(GP);

        let five_plain: Vec<&Person> = vec![&plain; 5];
        let r = validate_palanquee(
            PalanqueeKind::Exploration,
            20,
            Some(&guide),
            &five_plain,
            on(),
        );
        assert!(r
            .violations
            .iter()
            .any(|v| matches!(v, PalanqueeViolation::TooManyGuidedMembers { count: 5 })));

        let with_gp: Vec<&Person> = vec![&plain, &plain, &plain, &plain, &extra_gp];
        let r = validate_palanquee(PalanqueeKind::Exploration, 20, Some(&guide), &with_gp, on());
        assert!(r.is_legal(), "{:?}", r.violations);

        // No +1 slot beyond 40 m.
        let n3 = adult(N3);
        let gp_extra = adult(GP);
        let deep: Vec<&Person> = vec![&n3, &n3, &n3, &n3, &gp_extra];
        let e4 = adult(E4);
        let r = validate_palanquee(PalanqueeKind::Exploration, 60, Some(&e4), &deep, on());
        assert!(r
            .violations
            .iter()
            .any(|v| matches!(v, PalanqueeViolation::NoExtraSlotBeyond40m { count: 5 })));

        let six: Vec<&Person> = vec![&plain; 6];
        let r = validate_palanquee(PalanqueeKind::Exploration, 20, Some(&guide), &six, on());
        assert!(!r.is_legal());
    }

    #[test]
    fn bapteme_takes_exactly_one_student_with_an_e1() {
        let e1 = adult(&[C::Initiateur]);
        let newbie = adult(NOBODY);
        let r = validate_palanquee(PalanqueeKind::Bapteme, 6, Some(&e1), &[&newbie], on());
        assert!(r.is_legal(), "{:?}", r.violations);

        let r = validate_palanquee(
            PalanqueeKind::Bapteme,
            6,
            Some(&e1),
            &[&newbie, &newbie],
            on(),
        );
        assert!(r
            .violations
            .iter()
            .any(|v| matches!(v, PalanqueeViolation::BaptemeSingleStudent { count: 2 })));

        // A plain GP cannot run a baptême in teaching terms… but CAN guide
        // débutants in exploration (III-16b row 0-6 m).
        let gp = adult(GP);
        let r = validate_palanquee(PalanqueeKind::Bapteme, 6, Some(&gp), &[&newbie], on());
        assert!(!r.is_legal());
        let r = validate_palanquee(PalanqueeKind::Exploration, 6, Some(&gp), &[&newbie], on());
        assert!(r.is_legal(), "{:?}", r.violations);
    }

    #[test]
    fn teaching_requires_the_grade_of_the_space() {
        let e1 = adult(&[C::Initiateur]);
        let e2 = adult(&[C::Initiateur, C::N4Gp]);
        let e3 = adult(&[C::Mf1]);
        let student = adult(N1);

        // E1 teaches 0-6 m but not 0-20 m.
        let r = validate_palanquee(PalanqueeKind::Teaching, 6, Some(&e1), &[&student], on());
        assert!(r.is_legal(), "{:?}", r.violations);
        let r = validate_palanquee(PalanqueeKind::Teaching, 20, Some(&e1), &[&student], on());
        assert!(!r.is_legal());
        // E2 teaches 0-20 m but not 0-40 m; E3 does.
        let r = validate_palanquee(PalanqueeKind::Teaching, 20, Some(&e2), &[&student], on());
        assert!(r.is_legal(), "{:?}", r.violations);
        let n2 = adult(N2);
        let r = validate_palanquee(PalanqueeKind::Teaching, 40, Some(&e2), &[&n2], on());
        assert!(!r.is_legal());
        let r = validate_palanquee(PalanqueeKind::Teaching, 40, Some(&e3), &[&n2], on());
        assert!(r.is_legal(), "{:?}", r.violations);
    }

    #[test]
    fn autonomous_groups_are_2_to_3_qualified_adults() {
        let n2 = adult(N2);
        let r = validate_palanquee(PalanqueeKind::Exploration, 20, None, &[&n2, &n2], on());
        assert!(r.is_legal(), "{:?}", r.violations);
        let r = validate_palanquee(PalanqueeKind::Exploration, 20, None, &[&n2], on());
        assert!(r
            .violations
            .iter()
            .any(|v| matches!(v, PalanqueeViolation::AutonomousGroupSize { count: 1 })));
        let r = validate_palanquee(
            PalanqueeKind::Exploration,
            20,
            None,
            &[&n2, &n2, &n2, &n2],
            on(),
        );
        assert!(!r.is_legal());

        // PA20 does not allow autonomy at 40 m.
        let r = validate_palanquee(PalanqueeKind::Exploration, 40, None, &[&n2, &n2], on());
        assert_eq!(r.violations.len(), 2);

        // A 16-year-old N2 may be autonomous at 20 m — with formalities.
        let minor = Person {
            certifications: N2,
            statuses: &[],
            birthdate: d(2010, 1, 1), // 16
        };
        let n2_adult = adult(N2);
        let r = validate_palanquee(
            PalanqueeKind::Exploration,
            20,
            None,
            &[&minor, &n2_adult],
            on(),
        );
        assert!(r.is_legal(), "{:?}", r.violations);
        assert!(r.warnings.iter().any(|w| matches!(
            w,
            PalanqueeWarning::MinorAutonomyFormalities { member_index: 0 }
        )));

        // The same minor at 15 (before the birthday) cannot.
        let r = validate_palanquee(
            PalanqueeKind::Exploration,
            20,
            None,
            &[&minor, &n2_adult],
            d(2025, 12, 31),
        );
        assert!(!r.is_legal());
    }

    #[test]
    fn pesh_members_trigger_handisub_warnings_not_violations() {
        let e2 = adult(&[C::Initiateur, C::N4Gp]);
        let pesh = adult(&[C::Pesh20]);
        let r = validate_palanquee(PalanqueeKind::Exploration, 20, Some(&e2), &[&pesh], on());
        assert!(r.is_legal(), "{:?}", r.violations);
        assert!(r.warnings.iter().any(|w| matches!(
            w,
            PalanqueeWarning::HandisubQualificationCheck { member_index: 0 }
        )));

        let pesh40 = adult(&[C::Pesh40]);
        let e3 = adult(&[C::Mf1]);
        let r = validate_palanquee(PalanqueeKind::Exploration, 40, Some(&e3), &[&pesh40], on());
        assert!(r.is_legal(), "{:?}", r.violations);
        assert!(r
            .warnings
            .contains(&PalanqueeWarning::Pesh40ClinicalProfile));

        // PESH are never autonomous.
        let r = validate_palanquee(PalanqueeKind::Exploration, 12, None, &[&pesh, &pesh], on());
        assert!(!r.is_legal());
    }

    /// H1: formation dives go BEYOND held aptitudes by definition — a
    /// PE20 student in PE40 formation dives to 40 m under an E3
    /// (art. A322-85, annexe III-16a). Warning, never a violation.
    #[test]
    fn teaching_lets_students_train_beyond_their_held_aptitude() {
        let e3 = adult(&[C::Mf1]);
        let n1_student = adult(N1); // holds PE20 only
        let r = validate_palanquee(PalanqueeKind::Teaching, 40, Some(&e3), &[&n1_student], on());
        assert!(r.is_legal(), "{:?}", r.violations);
        assert!(r.warnings.iter().any(|w| matches!(
            w,
            PalanqueeWarning::TrainingBeyondHeldAptitude { member_index: 0 }
        )));
        // Exploration still requires the held aptitude.
        let gp = adult(GP);
        let r = validate_palanquee(
            PalanqueeKind::Exploration,
            40,
            Some(&gp),
            &[&n1_student],
            on(),
        );
        assert!(!r.is_legal());
    }

    /// M2: autonomy-specific restrictions are silent on guided dives.
    #[test]
    fn guided_dives_do_not_surface_autonomy_restrictions() {
        let minor_n2 = Person {
            certifications: N2,
            statuses: &[],
            birthdate: d(2010, 1, 1), // 16
        };
        let gp = adult(GP);
        let r = validate_palanquee(
            PalanqueeKind::Exploration,
            20,
            Some(&gp),
            &[&minor_n2],
            on(),
        );
        assert!(r.is_legal(), "{:?}", r.violations);
        assert!(
            !r.warnings.iter().any(|w| matches!(
                w,
                PalanqueeWarning::MemberRestriction {
                    restriction: crate::ExerciseRestriction::MinorAutonomyAirNitroxOnly,
                    ..
                } | PalanqueeWarning::MinorAutonomyFormalities { .. }
            )),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn depth_beyond_sixty_is_out_of_air_nitrox_scope() {
        let e4 = adult(E4);
        let n3 = adult(N3);
        let r = validate_palanquee(PalanqueeKind::Exploration, 61, Some(&e4), &[&n3], on());
        assert_eq!(
            r.violations,
            vec![PalanqueeViolation::DepthExceedsAirNitroxLimit { depth_m: 61 }]
        );
    }
}
