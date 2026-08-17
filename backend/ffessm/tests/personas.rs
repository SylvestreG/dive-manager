//! End-to-end personas checked against the MFT sheets in force — realistic
//! club members exercised through the whole public API.

use chrono::NaiveDate;
use ffessm::{
    aptitudes, can_be_dp, check_eligibility, eligible_next, exercisable_aptitudes, supervision,
    validate_palanquee, Certification as C, DiverAptitude as A, DpContext, EligibilityIssue,
    PalanqueeKind, Person, SupervisionGrade, TrainingStatus,
};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
fn today() -> NaiveDate {
    d(2026, 8, 17)
}

#[test]
fn persona_gold_youth_diver_12_years_old() {
    // Léa, 12 ans, plongeuse d'or depuis ses 11 ans.
    let lea = Person {
        certifications: &[C::PlongeurOr],
        statuses: &[],
        birthdate: d(2014, 3, 10),
    };
    let apt = aptitudes(&lea, today());
    assert!(apt.diver.contains(&A::Pe20), "Or à 12 ans = espace 0-20 m");
    // Elle peut viser le N1 (12 ans) mais pas le N2 (15 ans).
    let next = eligible_next(&lea, today());
    assert!(next.contains(&C::N1));
    assert!(!next.contains(&C::N2));
}

#[test]
fn persona_teen_n2_and_the_autonomy_gates() {
    // Tom, N2 obtenu à 15 ans : PA20 détenu mais exerçable à 16 seulement.
    let tom = Person {
        certifications: &[C::N1, C::N2],
        statuses: &[],
        birthdate: d(2011, 2, 1),
    };
    let at_15 = d(2026, 1, 31);
    let at_16 = d(2027, 2, 1);
    assert!(aptitudes(&tom, at_15).diver.contains(&A::Pa20));
    assert!(!exercisable_aptitudes(&tom, at_15)
        .0
        .diver
        .contains(&A::Pa20));
    assert!(exercisable_aptitudes(&tom, at_16)
        .0
        .diver
        .contains(&A::Pa20));
    // Encadré à 40 m dès l'obtention (PE40), même mineur.
    assert!(exercisable_aptitudes(&tom, at_15).0.has(A::Pe40));
}

#[test]
fn persona_n3_adult_full_prerogatives() {
    // Karim, 25 ans, N3 + Nitrox confirmé.
    let karim = Person {
        certifications: &[C::N1, C::N2, C::N3, C::Nitrox, C::NitroxConfirme, C::Rifap],
        statuses: &[],
        birthdate: d(2001, 5, 5),
    };
    let (apt, restrictions) = exercisable_aptitudes(&karim, today());
    assert!(apt.diver.contains(&A::Pa60));
    assert!(restrictions.is_empty());
    assert_eq!(ffessm::max_o2_fraction(&karim), 1.0);
    // Il peut viser le N4 (N3 + RIFAP) mais pas le MF1 (GP requis).
    let next = eligible_next(&karim, today());
    assert!(next.contains(&C::N4Gp));
    assert!(!next.contains(&C::Mf1));
    // Deux N3 autonomes à 40 m : palanquée légale.
    let buddy = karim;
    let report = validate_palanquee(
        PalanqueeKind::Exploration,
        40,
        None,
        &[&karim, &buddy],
        today(),
    );
    assert!(report.is_legal(), "{:?}", report.violations);
}

#[test]
fn persona_initiateur_gp_is_e2_and_club_pool_dp() {
    // Sophie, Initiatrice + GP-N4, 30 ans : E2, DP piscine, pas DP explo.
    let sophie = Person {
        certifications: &[C::N1, C::N2, C::N3, C::N4Gp, C::Initiateur, C::Rifap],
        statuses: &[],
        birthdate: d(1996, 9, 9),
    };
    let cap = supervision(&sophie, today());
    assert!(cap.gp);
    assert_eq!(cap.teaching, Some(SupervisionGrade::E2));
    assert!(can_be_dp(&sophie, DpContext::PoolMax6m, false, today()));
    assert!(!can_be_dp(
        &sophie,
        DpContext::ExplorationAirNitrox,
        false,
        today()
    ));
    // En passant DP-N5 elle deviendrait DP exploration.
    let issues = check_eligibility(C::N5Dpe, &sophie, today());
    assert!(issues
        .iter()
        .all(|i| matches!(i, EligibilityIssue::NeedsManualCheck(_))));
}

#[test]
fn persona_mf1_trainee_keeps_e2_only_while_livret_lives() {
    let statuses = [TrainingStatus::StagiairePedagogiqueMf1 {
        livret_valid_until: d(2027, 1, 1),
    }];
    let marc = Person {
        certifications: &[C::N4Gp, C::Rifap],
        statuses: &statuses,
        birthdate: d(1998, 4, 4),
    };
    assert_eq!(
        supervision(&marc, today()).teaching,
        Some(SupervisionGrade::E2)
    );
    assert_eq!(supervision(&marc, d(2027, 6, 1)).teaching, None);
    // Le stagiaire n'est jamais DP.
    assert!(!can_be_dp(&marc, DpContext::PoolMax6m, false, today()));
}

#[test]
fn persona_mf1_runs_the_club() {
    // Ana, MF1 + Nitrox confirmé : E3, DP partout en air/nitrox.
    let ana = Person {
        certifications: &[C::Mf1, C::NitroxConfirme, C::Rifap, C::PermisCotier],
        statuses: &[],
        birthdate: d(1985, 12, 25),
    };
    assert_eq!(
        supervision(&ana, today()).teaching,
        Some(SupervisionGrade::E3)
    );
    assert!(can_be_dp(
        &ana,
        DpContext::ExplorationAirNitrox,
        true,
        today()
    ));
    assert!(can_be_dp(&ana, DpContext::Teaching, false, today()));
    assert!(!can_be_dp(&ana, DpContext::TrimixAll, false, today()));
    // Elle enseigne à 40 m mais pas à 60 m.
    let n2 = Person {
        certifications: &[C::N2],
        statuses: &[],
        birthdate: d(2000, 1, 1),
    };
    let ok = validate_palanquee(PalanqueeKind::Teaching, 40, Some(&ana), &[&n2], today());
    assert!(ok.is_legal(), "{:?}", ok.violations);
    let n3 = Person {
        certifications: &[C::N3],
        statuses: &[],
        birthdate: d(2000, 1, 1),
    };
    let deep = validate_palanquee(PalanqueeKind::Teaching, 60, Some(&ana), &[&n3], today());
    assert!(!deep.is_legal());
}

#[test]
fn persona_pesh_20_dives_guided_with_handisub_warning() {
    let pesh = Person {
        certifications: &[C::Pesh20, C::Nitrox],
        statuses: &[],
        birthdate: d(1999, 7, 7),
    };
    // PESH-20 a ouvert la qualification nitrox élémentaire.
    assert_eq!(ffessm::max_o2_fraction(&pesh), 0.40);
    let guide = Person {
        certifications: &[C::Initiateur, C::N4Gp, C::Eh1],
        statuses: &[],
        birthdate: d(1990, 1, 1),
    };
    let report = validate_palanquee(
        PalanqueeKind::Exploration,
        20,
        Some(&guide),
        &[&pesh],
        today(),
    );
    assert!(report.is_legal(), "{:?}", report.violations);
    assert!(!report.warnings.is_empty(), "Handisub check surfaced");
}
