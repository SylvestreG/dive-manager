use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// In-process fixed-window rate limiter. Sufficient for a single-instance
/// deployment (ADR-0001: one VPS, no shared infrastructure).
#[derive(Default)]
pub struct RateLimiter {
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// Records a hit for `key` and returns whether it is within `max` hits
    /// per `window`.
    pub fn check(&self, key: &str, max: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter poisoned");
        // Opportunistic global cleanup so the map cannot grow unbounded.
        if hits.len() > 50_000 {
            hits.retain(|_, v| v.iter().any(|t| now.duration_since(*t) < window));
        }
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < window);
        if entry.len() >= max {
            return false;
        }
        entry.push(now);
        true
    }

    /// Clears a key's window (e.g. after a successful login).
    pub fn reset(&self, key: &str) {
        self.hits.lock().expect("rate limiter poisoned").remove(key);
    }
}
