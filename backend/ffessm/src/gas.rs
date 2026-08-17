//! Gas rules (Code du Sport art. A322-76 / A322-92, MFT Nitrox May 2024).

use crate::{Certification, Person};

/// Legal partial pressure of inspired oxygen underwater (art. A322-92).
pub const PPO2_MIN_BAR: f32 = 0.16;
pub const PPO2_MAX_BAR: f32 = 1.6;

/// Maximum O2 fraction the person may breathe (MFT Nitrox May 2024):
/// none → air (0.21); PN → 0.40; PN-C / trimix qualifications → 1.0
/// (pure O2 for decompression).
pub fn max_o2_fraction(person: &Person) -> f32 {
    if person.holds_any(&[
        Certification::NitroxConfirme,
        Certification::TrimixElementaire,
        Certification::Trimix,
    ]) {
        1.0
    } else if person.holds(Certification::Nitrox) {
        0.40
    } else {
        0.21
    }
}

/// Maximum operating depth in meters for a given O2 fraction and PpO2 limit,
/// using the standard 1 bar / 10 m model: MOD = (PpO2 / fO2 - 1) × 10.
/// The PpO2 limit is clamped to the legal maximum (1.6 bar).
pub fn nitrox_mod_meters(o2_fraction: f32, ppo2_limit_bar: f32) -> f32 {
    let ppo2 = ppo2_limit_bar.clamp(PPO2_MIN_BAR, PPO2_MAX_BAR);
    let o2 = o2_fraction.clamp(0.01, 1.0);
    ((ppo2 / o2) - 1.0) * 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn adult(certs: &'static [Certification]) -> Person<'static> {
        Person {
            certifications: certs,
            statuses: &[],
            birthdate: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
        }
    }

    #[test]
    fn o2_fraction_ladder() {
        assert_eq!(max_o2_fraction(&adult(&[])), 0.21);
        assert_eq!(max_o2_fraction(&adult(&[Certification::Nitrox])), 0.40);
        assert_eq!(
            max_o2_fraction(&adult(&[Certification::NitroxConfirme])),
            1.0
        );
        assert_eq!(max_o2_fraction(&adult(&[Certification::Trimix])), 1.0);
    }

    #[test]
    fn mod_reference_values() {
        // Nx32 at PpO2 1.4: 33.75 m; Nx40 at 1.6: 30 m; pure O2 at 1.6: 6 m.
        assert!((nitrox_mod_meters(0.32, 1.4) - 33.75).abs() < 0.01);
        assert!((nitrox_mod_meters(0.40, 1.6) - 30.0).abs() < 0.01);
        assert!((nitrox_mod_meters(1.0, 1.6) - 6.0).abs() < 0.01);
        // PpO2 requests above the legal maximum are clamped to 1.6 bar.
        assert!((nitrox_mod_meters(1.0, 2.0) - 6.0).abs() < 0.01);
    }
}
