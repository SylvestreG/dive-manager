//! Read-only FFESSM certification catalog: the frozen code list with, for
//! each entry, everything holding it makes acquired (`ffessm::covered_by`).
//! Powers the club-side checkbox tree where checking "N2" auto-ticks
//! N1/PE12/PA12/PE40/PA20 as derived coverage.

use axum::Json;
use serde::Serialize;

use crate::identity::extractors::CurrentUser;

#[derive(Debug, Serialize)]
pub struct CatalogEntryDto {
    pub code: &'static str,
    pub label: &'static str,
    /// Codes of every certification this one covers (proven chain +
    /// prerogative coverage) — person-independent, derived, never stored.
    pub covers: Vec<&'static str>,
}

/// `GET /catalog/certifications` — static domain data, any authenticated
/// user may read it (no tenant, no permission: nothing personal inside).
pub async fn list_catalog(CurrentUser(_auth): CurrentUser) -> Json<Vec<CatalogEntryDto>> {
    Json(
        ffessm::Certification::ALL
            .into_iter()
            .map(|cert| CatalogEntryDto {
                code: cert.code(),
                label: cert.label_fr(),
                covers: ffessm::covered_by(cert)
                    .iter()
                    .map(|covered| covered.code())
                    .collect(),
            })
            .collect(),
    )
}
