//! The minimal view of a person the library needs. Storage (dates of issue,
//! who signed, etc.) belongs to the `members` module.

use chrono::NaiveDate;

use crate::Certification;

/// Dated transitional statuses that are NOT certifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainingStatus {
    /// Stagiaire pédagogique MF1: grants E2 while the livret is valid
    /// (3 years — MFT MF1 Jul 2026; Code du Sport annexe III-15b).
    StagiairePedagogiqueMf1 { livret_valid_until: NaiveDate },
}

#[derive(Debug, Clone, Copy)]
pub struct Person<'a> {
    pub certifications: &'a [Certification],
    pub statuses: &'a [TrainingStatus],
    pub birthdate: NaiveDate,
}

impl Person<'_> {
    pub fn holds(&self, cert: Certification) -> bool {
        self.certifications.contains(&cert)
    }

    pub fn holds_any(&self, certs: &[Certification]) -> bool {
        certs.iter().any(|c| self.holds(*c))
    }

    pub fn age_on(&self, date: NaiveDate) -> u8 {
        age_on(self.birthdate, date)
    }

    pub fn is_minor_on(&self, date: NaiveDate) -> bool {
        self.age_on(date) < 18
    }
}

/// Full years between `birthdate` and `date` (0 if `date` precedes it).
pub fn age_on(birthdate: NaiveDate, date: NaiveDate) -> u8 {
    use chrono::Datelike;
    if date < birthdate {
        return 0;
    }
    let mut age = date.year() - birthdate.year();
    if (date.month(), date.day()) < (birthdate.month(), birthdate.day()) {
        age -= 1;
    }
    age.clamp(0, u8::MAX as i32) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn age_flips_exactly_on_the_birthday() {
        let birth = d(2010, 6, 15);
        assert_eq!(age_on(birth, d(2026, 6, 14)), 15);
        assert_eq!(age_on(birth, d(2026, 6, 15)), 16);
        assert_eq!(age_on(birth, d(2026, 6, 16)), 16);
        assert_eq!(age_on(birth, d(2009, 1, 1)), 0);
    }

    #[test]
    fn leap_day_birthdays_flip_on_march_first_in_common_years() {
        let birth = d(2012, 2, 29);
        assert_eq!(age_on(birth, d(2026, 2, 28)), 13);
        assert_eq!(age_on(birth, d(2026, 3, 1)), 14);
    }
}
