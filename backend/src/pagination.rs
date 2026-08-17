use serde::{Deserialize, Serialize};

pub const DEFAULT_PER_PAGE: u64 = 50;
pub const MAX_PER_PAGE: u64 = 200;

/// Query-string pagination: `?page=1&per_page=50`, 1-based, clamped.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct PageParams {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

/// Standard paginated response envelope: `{ items, total, page, per_page }`.
#[derive(Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    /// 1-based.
    pub page: u64,
    pub per_page: u64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, zero_based_page: u64, per_page: u64) -> Self {
        Page {
            items,
            total,
            page: zero_based_page + 1,
            per_page,
        }
    }
}

impl PageParams {
    /// Returns (zero-based page index, page size).
    pub fn resolve(&self) -> (u64, u64) {
        let page = self.page.unwrap_or(1).max(1) - 1;
        let per_page = self
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);
        (page, per_page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_clamps_and_defaults() {
        assert_eq!(PageParams::default().resolve(), (0, DEFAULT_PER_PAGE));
        let p = PageParams {
            page: Some(0),
            per_page: Some(10_000),
        };
        assert_eq!(p.resolve(), (0, MAX_PER_PAGE));
        let p = PageParams {
            page: Some(3),
            per_page: Some(25),
        };
        assert_eq!(p.resolve(), (2, 25));
    }
}
