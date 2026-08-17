//! Matrix 1 & 2: certifications → aptitudes held, and aptitudes actually
//! EXERCISABLE at a given date (age gates of Code du Sport art. A322-73 /
//! A322-89 and the MFT sheets).

use std::collections::BTreeSet;

use chrono::NaiveDate;

use crate::{Certification, DiverAptitude, GasAptitude, Person};

/// Scuba diving starts at 8 (MFT "Jeunes plongeurs", Aug 2023).
pub const SCUBA_MIN_AGE: u8 = 8;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Aptitudes {
    pub diver: BTreeSet<DiverAptitude>,
    pub gas: BTreeSet<GasAptitude>,
}

impl Aptitudes {
    /// Deepest guided (PE/débutant) depth allowed, if any.
    pub fn max_guided_depth_m(&self) -> Option<u8> {
        self.diver
            .iter()
            .filter(|a| !a.is_autonomous() && !a.is_pesh())
            .map(|a| a.max_depth_m())
            .max()
    }

    /// Deepest autonomous (PA) depth allowed, if any.
    pub fn max_autonomous_depth_m(&self) -> Option<u8> {
        self.diver
            .iter()
            .filter(|a| a.is_autonomous())
            .map(|a| a.max_depth_m())
            .max()
    }

    pub fn has(&self, aptitude: DiverAptitude) -> bool {
        self.diver.iter().any(|a| a.covers(aptitude))
    }
}

/// Aptitudes CONFERRED by the held certifications (annexe III-14b + MFT
/// sheets). Age only matters for the youth Plongeur d'Or space and the
/// global scuba minimum age.
pub fn aptitudes(person: &Person, on: NaiveDate) -> Aptitudes {
    let mut result = Aptitudes::default();
    let age = person.age_on(on);
    if age < SCUBA_MIN_AGE {
        return result;
    }

    // Anyone of scuba age may dive as débutant (baptême, annexe III-16b).
    result.diver.insert(DiverAptitude::Debutant);
    result.gas.insert(GasAptitude::Air);

    for cert in person.certifications {
        use Certification as C;
        use DiverAptitude as A;
        match cert {
            // Youth (MFT Jeunes plongeurs, Aug 2023): bronze/argent 0-6 m;
            // or: 0-12 m at 10-12, 0-20 m from 12.
            C::PlongeurBronze | C::PlongeurArgent => {}
            C::PlongeurOr => {
                result
                    .diver
                    .insert(if age < 12 { A::Pe12 } else { A::Pe20 });
            }
            // Main pathway (annexe III-14b + MFT N1 Dec 2025 / N2 May 2026 /
            // N3 Dec 2025). N1 confers PE20 ONLY (PA12 is separate).
            C::Pe12 => {
                result.diver.insert(A::Pe12);
            }
            C::N1 => {
                result.diver.insert(A::Pe20);
            }
            C::Pa12 => {
                result.diver.insert(A::Pa12);
            }
            C::Pe40 => {
                result.diver.insert(A::Pe40);
            }
            C::Pa20 => {
                result.diver.insert(A::Pa20);
            }
            C::N2 => {
                result.diver.insert(A::Pa20);
                result.diver.insert(A::Pe40);
            }
            C::Pa40 => {
                result.diver.insert(A::Pa40);
            }
            C::Pe60 => {
                result.diver.insert(A::Pe60);
            }
            // N3 = PA40 + PE60 + PA60 ("PA60-Niveau 3", MFT N3 Dec 2025).
            // N4/N5/MF1/MF2 require N3 in their chain: same diver base.
            C::N3 | C::N4Gp | C::N5Dpe | C::Mf1 | C::Mf2 => {
                result.diver.insert(A::Pa40);
                result.diver.insert(A::Pe60);
                result.diver.insert(A::Pa60);
            }
            // Handisub (MFT PESH, May 2025) — never autonomous.
            C::Pesh6 => {
                result.diver.insert(A::Pesh6);
            }
            C::Pesh12 => {
                result.diver.insert(A::Pesh12);
            }
            C::Pesh20 => {
                result.diver.insert(A::Pesh20);
            }
            C::Pesh40 => {
                result.diver.insert(A::Pesh40);
            }
            // Gas (MFT Nitrox May 2024 / Trimix Oct 2017).
            C::Nitrox => {
                result.gas.insert(GasAptitude::Nitrox40);
            }
            C::NitroxConfirme => {
                result.gas.insert(GasAptitude::Nitrox40);
                result.gas.insert(GasAptitude::NitroxAll);
            }
            C::TrimixElementaire => {
                result.gas.insert(GasAptitude::Pth70);
            }
            C::Trimix => {
                result.gas.insert(GasAptitude::Pth70);
                result.gas.insert(GasAptitude::Pth120);
            }
            // Supervision certifications and state diplomas confer no diver
            // aptitude by themselves (aptitudes are justified per A322-77);
            // club-life qualifications neither.
            C::Initiateur
            | C::Beppa
            | C::Eh1
            | C::Eh2
            | C::Bpjeps
            | C::Bees1
            | C::Bees2
            | C::Dejeps
            | C::DejepsPlongeeProfonde
            | C::Desjeps
            | C::Rifap
            | C::Tiv
            | C::PermisCotier => {}
        }
    }
    result
}

/// Structured restrictions attached to exercisable aptitudes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExerciseRestriction {
    /// A322-73 (2025-10-17): minors' autonomy is limited to air and nitrox.
    MinorAutonomyAirNitroxOnly,
    /// MFT N2/N3: minor autonomy requires the informed authorization of the
    /// legal guardian and informing the palanquée.
    MinorAutonomyFormalities,
    /// MFT N2 (May 2026): PE40 holders under 16 — no mandatory-deco dives,
    /// max 2 dives/day at least 3 h apart, 2nd dive <= 20 m if 1st > 30 m.
    Pe40Under16Profile,
    /// MFT Jeunes plongeurs: no-deco dives only, one dive per day under 12.
    YouthUnder12Profile,
    /// MFT Handisub: PESH40 clinical dive profile (max depth reached first,
    /// <= 40 min, no mandatory stops, end of dive around 6 m).
    Pesh40ClinicalProfile,
}

impl ExerciseRestriction {
    pub fn label_fr(&self) -> &'static str {
        match self {
            ExerciseRestriction::MinorAutonomyAirNitroxOnly => {
                "Mineur : autonomie limitée à l'air et au nitrox"
            }
            ExerciseRestriction::MinorAutonomyFormalities => {
                "Mineur autonome : autorisation éclairée du responsable légal et information de la palanquée"
            }
            ExerciseRestriction::Pe40Under16Profile => {
                "PE40 avant 16 ans : plongées sans palier obligatoire, 2 plongées/jour maximum espacées de 3 h, 2e plongée ≤ 20 m si la 1re a dépassé 30 m"
            }
            ExerciseRestriction::YouthUnder12Profile => {
                "Moins de 12 ans : plongées dans la courbe de sécurité, une plongée par jour"
            }
            ExerciseRestriction::Pesh40ClinicalProfile => {
                "PESH40 : profondeur maximale en début de plongée, durée ≤ 40 min, sans palier obligatoire, fin de plongée vers 6 m"
            }
        }
    }
}

/// Aptitudes actually EXERCISABLE on `on`, applying the age-of-exercise
/// gates: PA12/PA20 from 16, PA40 from 17 (A322-73), PA60 from 18
/// (A322-89 + MFT N3). Returns the applicable structured restrictions.
pub fn exercisable_aptitudes(
    person: &Person,
    on: NaiveDate,
) -> (Aptitudes, Vec<ExerciseRestriction>) {
    let mut held = aptitudes(person, on);
    let age = person.age_on(on);
    let mut restrictions = Vec::new();

    held.diver.retain(|a| match a {
        DiverAptitude::Pa12 | DiverAptitude::Pa20 => age >= 16,
        DiverAptitude::Pa40 => age >= 17,
        DiverAptitude::Pa60 => age >= 18,
        _ => true,
    });

    if age < 18 && held.diver.iter().any(|a| a.is_autonomous()) {
        restrictions.push(ExerciseRestriction::MinorAutonomyAirNitroxOnly);
        restrictions.push(ExerciseRestriction::MinorAutonomyFormalities);
    }
    if age < 16 && held.has(DiverAptitude::Pe40) {
        restrictions.push(ExerciseRestriction::Pe40Under16Profile);
    }
    if age < 12 {
        restrictions.push(ExerciseRestriction::YouthUnder12Profile);
    }
    if held.diver.contains(&DiverAptitude::Pesh40) {
        restrictions.push(ExerciseRestriction::Pesh40ClinicalProfile);
    }

    (held, restrictions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Certification as C;
    use DiverAptitude as A;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn person<'a>(certs: &'a [C], birth: NaiveDate) -> Person<'a> {
        Person {
            certifications: certs,
            statuses: &[],
            birthdate: birth,
        }
    }
    fn on() -> NaiveDate {
        d(2026, 8, 17)
    }
    fn born_years_ago(years: i32) -> NaiveDate {
        d(2026 - years, 1, 1) // birthday already passed in 2026
    }

    #[test]
    fn certification_to_aptitude_matrix_matches_annexe_iii_14b() {
        let cases: &[(&[C], &[A])] = &[
            (&[], &[A::Debutant]),
            (&[C::Pe12], &[A::Debutant, A::Pe12]),
            (&[C::N1], &[A::Debutant, A::Pe20]),
            // N1 does NOT include PA12.
            (&[C::N1, C::Pa12], &[A::Debutant, A::Pe20, A::Pa12]),
            (&[C::N2], &[A::Debutant, A::Pa20, A::Pe40]),
            (&[C::N3], &[A::Debutant, A::Pa40, A::Pe60, A::Pa60]),
            (&[C::N4Gp], &[A::Debutant, A::Pa40, A::Pe60, A::Pa60]),
            (&[C::Pesh20], &[A::Debutant, A::Pesh20]),
            // Supervision-only certifications confer no diver aptitude.
            (&[C::Initiateur], &[A::Debutant]),
            (&[C::Bees1], &[A::Debutant]),
        ];
        for (certs, expected) in cases {
            let p = person(certs, born_years_ago(30));
            let got = aptitudes(&p, on());
            let expected: BTreeSet<A> = expected.iter().copied().collect();
            assert_eq!(got.diver, expected, "certs {certs:?}");
        }
    }

    #[test]
    fn gas_matrix() {
        let p = person(&[C::N2, C::Nitrox], born_years_ago(30));
        let a = aptitudes(&p, on());
        assert!(a.gas.contains(&GasAptitude::Nitrox40));
        assert!(!a.gas.contains(&GasAptitude::NitroxAll));
        let p = person(&[C::N2, C::NitroxConfirme], born_years_ago(30));
        assert!(aptitudes(&p, on()).gas.contains(&GasAptitude::NitroxAll));
        let p = person(&[C::Trimix], born_years_ago(30));
        let a = aptitudes(&p, on());
        assert!(a.gas.contains(&GasAptitude::Pth70) && a.gas.contains(&GasAptitude::Pth120));
    }

    #[test]
    fn under_eight_has_no_aptitude_at_all() {
        let p = person(&[C::N1], born_years_ago(7));
        assert!(aptitudes(&p, on()).diver.is_empty());
    }

    #[test]
    fn plongeur_or_space_depends_on_age() {
        let p = person(&[C::PlongeurOr], born_years_ago(11));
        assert!(aptitudes(&p, on()).diver.contains(&A::Pe12));
        let p = person(&[C::PlongeurOr], born_years_ago(12));
        assert!(aptitudes(&p, on()).diver.contains(&A::Pe20));
    }

    /// The core "holding ≠ exercising" rule, tested at every age boundary,
    /// the day before and the day of the birthday.
    #[test]
    fn autonomy_exercise_age_gates() {
        let birth = d(2010, 6, 15);
        let day_before_16 = d(2026, 6, 14);
        let at_16 = d(2026, 6, 15);
        let p = person(&[C::N1, C::Pa20], birth);

        assert!(aptitudes(&p, day_before_16).diver.contains(&A::Pa20));
        let (ex, _) = exercisable_aptitudes(&p, day_before_16);
        assert!(
            !ex.diver.contains(&A::Pa20),
            "held but not exercisable at 15"
        );
        let (ex, restrictions) = exercisable_aptitudes(&p, at_16);
        assert!(
            ex.diver.contains(&A::Pa20),
            "exercisable on the 16th birthday"
        );
        assert!(restrictions.contains(&ExerciseRestriction::MinorAutonomyAirNitroxOnly));
        assert!(restrictions.contains(&ExerciseRestriction::MinorAutonomyFormalities));

        // PA40: 17. PA60 (via N3): 18.
        let p = person(&[C::N3], d(2009, 6, 15));
        let (ex, _) = exercisable_aptitudes(&p, d(2026, 6, 14)); // 16 turning 17
        assert!(!ex.diver.contains(&A::Pa40));
        let (ex, _) = exercisable_aptitudes(&p, d(2026, 6, 15)); // 17
        assert!(ex.diver.contains(&A::Pa40));
        assert!(!ex.diver.contains(&A::Pa60), "PA60 needs 18");
        assert!(ex.diver.contains(&A::Pe60), "PE60 has no exercise gate");
        let (ex, restrictions) = exercisable_aptitudes(&p, d(2027, 6, 15)); // 18
        assert!(ex.diver.contains(&A::Pa60));
        assert!(restrictions.is_empty(), "adult N3: no restrictions");
    }

    #[test]
    fn pe40_under_16_and_youth_profiles() {
        let p = person(&[C::N1, C::Pe40], born_years_ago(15));
        let (ex, restrictions) = exercisable_aptitudes(&p, on());
        assert!(ex.has(A::Pe40));
        assert!(restrictions.contains(&ExerciseRestriction::Pe40Under16Profile));

        let p = person(&[C::PlongeurOr], born_years_ago(11));
        let (_, restrictions) = exercisable_aptitudes(&p, on());
        assert!(restrictions.contains(&ExerciseRestriction::YouthUnder12Profile));

        let p = person(&[C::Pesh40], born_years_ago(25));
        let (_, restrictions) = exercisable_aptitudes(&p, on());
        assert!(restrictions.contains(&ExerciseRestriction::Pesh40ClinicalProfile));
    }
}
