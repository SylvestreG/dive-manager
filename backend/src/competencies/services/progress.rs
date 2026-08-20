//! Pure progress computation. THE single percentage formula (weighted:
//! validated / total) — Fosse shipped two divergent client-side ones.

use serde::Serialize;

use crate::competencies::domain::ValidationStage;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Progress {
    pub total: u64,
    pub validated: u64,
    pub in_progress: u64,
    pub not_started: u64,
    /// Rounded percent of skills at the final stage; 0 when total is 0.
    pub percentage: u8,
}

impl Progress {
    pub fn zero() -> Progress {
        Progress {
            total: 0,
            validated: 0,
            in_progress: 0,
            not_started: 0,
            percentage: 0,
        }
    }

    /// Aggregates skill stages: `None` = not started, final = validated,
    /// anything else = in progress.
    pub fn of_stages<I: IntoIterator<Item = Option<ValidationStage>>>(stages: I) -> Progress {
        let mut progress = Progress::zero();
        for stage in stages {
            progress.total += 1;
            match stage {
                Some(stage) if stage.is_final() => progress.validated += 1,
                Some(_) => progress.in_progress += 1,
                None => progress.not_started += 1,
            }
        }
        progress.percentage = percentage(progress.validated, progress.total);
        progress
    }

    pub fn merge(items: impl IntoIterator<Item = Progress>) -> Progress {
        let mut merged = Progress::zero();
        for item in items {
            merged.total += item.total;
            merged.validated += item.validated;
            merged.in_progress += item.in_progress;
            merged.not_started += item.not_started;
        }
        merged.percentage = percentage(merged.validated, merged.total);
        merged
    }
}

fn percentage(validated: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((validated * 100 + total / 2) / total) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_is_zero_percent() {
        assert_eq!(Progress::of_stages([]), Progress::zero());
    }

    #[test]
    fn only_final_stages_count_as_validated() {
        let progress = Progress::of_stages([
            Some(ValidationStage::Valide),
            Some(ValidationStage::Vu),
            Some(ValidationStage::APerfectionner),
            None,
        ]);
        assert_eq!(progress.total, 4);
        assert_eq!(progress.validated, 1);
        assert_eq!(progress.in_progress, 2);
        assert_eq!(progress.not_started, 1);
        assert_eq!(progress.percentage, 25);
    }

    #[test]
    fn merge_is_weighted_not_a_mean_of_percentages() {
        // Fosse's bug: an unweighted mean of domain percentages would give
        // (100 + 0) / 2 = 50 here; the weighted result is 1/11 ≈ 9.
        let small = Progress::of_stages([Some(ValidationStage::Valide)]);
        let large = Progress::of_stages((0..10).map(|_| None));
        assert_eq!(Progress::merge([small, large]).percentage, 9);
    }
}
