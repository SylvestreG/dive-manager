//! Property-based invariants of the regulation model.

use chrono::NaiveDate;
use ffessm::{aptitudes, exercisable_aptitudes, Certification, Person};
use proptest::prelude::*;

fn arbitrary_certs() -> impl Strategy<Value = Vec<Certification>> {
    proptest::sample::subsequence(Certification::ALL.to_vec(), 0..Certification::ALL.len())
}

fn arbitrary_birthdate() -> impl Strategy<Value = NaiveDate> {
    (1940i32..2020, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

proptest! {
    /// Adding a certification never removes an aptitude.
    #[test]
    fn certifications_are_monotonic(
        certs in arbitrary_certs(),
        extra in proptest::sample::select(Certification::ALL.to_vec()),
        birth in arbitrary_birthdate(),
    ) {
        let on = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let base = Person { certifications: &certs, statuses: &[], birthdate: birth };
        let mut extended_certs = certs.clone();
        extended_certs.push(extra);
        let extended = Person { certifications: &extended_certs, statuses: &[], birthdate: birth };

        let before = aptitudes(&base, on);
        let after = aptitudes(&extended, on);
        prop_assert!(before.diver.is_subset(&after.diver));
        prop_assert!(before.gas.is_subset(&after.gas));
    }

    /// Getting older never removes an exercisable aptitude.
    #[test]
    fn aging_is_monotonic(
        certs in arbitrary_certs(),
        birth in arbitrary_birthdate(),
        years_later in 0i32..30,
    ) {
        let person = Person { certifications: &certs, statuses: &[], birthdate: birth };
        let start = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let later = NaiveDate::from_ymd_opt(2026 + years_later, 8, 17).unwrap();
        let (now, _) = exercisable_aptitudes(&person, start);
        let (future, _) = exercisable_aptitudes(&person, later);
        // Compared through covers(): the Plongeur d'Or upgrades from PE12 to
        // PE20 at 12 — deeper same-family aptitudes are progress, not loss.
        for aptitude in &now.diver {
            prop_assert!(
                future.diver.iter().any(|f| f.covers(*aptitude)),
                "{aptitude:?} lost between {now:?} and {future:?}"
            );
        }
    }

    /// Exercisable aptitudes are always a subset of held aptitudes.
    #[test]
    fn exercisable_is_a_subset_of_held(
        certs in arbitrary_certs(),
        birth in arbitrary_birthdate(),
    ) {
        let on = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let person = Person { certifications: &certs, statuses: &[], birthdate: birth };
        let held = aptitudes(&person, on);
        let (exercisable, _) = exercisable_aptitudes(&person, on);
        prop_assert!(exercisable.diver.is_subset(&held.diver));
    }

    /// Certification codes round-trip through parse for any certification.
    #[test]
    fn codes_round_trip(cert in proptest::sample::select(Certification::ALL.to_vec())) {
        prop_assert_eq!(Certification::parse(cert.code()), Some(cert));
    }
}
