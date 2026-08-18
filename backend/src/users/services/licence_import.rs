//! Self-service FFESSM licence import: fetch the official
//! `infolicencie.ffessm.fr` page and write profile fields + every mapped
//! brevet (provenance `imported`) for the ACCOUNT OWNER.
//!
//! Identity binding (review S1, 2026-08-18): licence QR codes are easy to
//! scan off someone else's phone, and imported grades feed real prerogatives
//! (including the derived member-visibility rule). The import is therefore
//! REFUSED unless the licence holder matches the account — every token of
//! the account's first and last names must appear in the holder name printed
//! on the licence, and the licence birthdate must match the profile's when
//! both are known. Fail-closed: an unreadable holder name refuses too.

use sea_orm::TransactionTrait;
use serde_json::json;

use crate::errors::AppError;
use crate::ffessm_licence::client::{fetch_licence_html, LicenceTarget, LookupError};
use crate::ffessm_licence::parse::{fold, parse_licence_page};
use crate::identity::entities::users;
use crate::identity::services::audit::{self, Actor};
use crate::users::entities::user_certifications::PROVENANCE_IMPORTED;
use crate::users::handlers::ffessm_certifications;
use crate::users::services::{certifications, profiles, training_statuses};
use crate::AppState;

pub struct ImportRequest<'a> {
    pub url: Option<&'a str>,
    pub number: Option<&'a str>,
    pub key: Option<&'a str>,
}

pub struct ImportOutcome {
    pub holder_name: Option<String>,
    pub licence_number: Option<String>,
    pub imported: Vec<String>,
    pub skipped_stronger: Vec<String>,
    pub unmapped: Vec<String>,
}

/// True when every token of `first` and `last` appears in the holder name
/// (accent/case-folded; the "Mr/Mme" civility never collides with a name
/// token requirement).
fn holder_matches(holder: &str, first: &str, last: &str) -> bool {
    let holder_tokens: Vec<String> = fold(holder).split(' ').map(str::to_string).collect();
    let required = |name: &str| {
        fold(name)
            .split(' ')
            .filter(|t| !t.is_empty())
            .all(|token| holder_tokens.iter().any(|h| h == token))
    };
    required(first) && required(last)
}

/// Runs the whole import for `user` (the authenticated account, never a
/// third party) inside one transaction. Rate limiting and the impersonation
/// guard belong to the HTTP handler.
pub async fn import_own_licence(
    state: &AppState,
    actor: Actor,
    user: &users::Model,
    request: ImportRequest<'_>,
) -> Result<ImportOutcome, AppError> {
    let target = LicenceTarget::resolve(request.url, request.number, request.key)?;
    let html = fetch_licence_html(&state.config.ffessm_infolicence_base_url, &target).await?;
    let preview = parse_licence_page(&html);
    if preview.looks_empty() {
        return Err(LookupError::NotFound.into());
    }

    // Identity binding — before ANY write.
    let holder_ok = preview
        .holder_name
        .as_deref()
        .is_some_and(|holder| holder_matches(holder, &user.first_name, &user.last_name));
    if !holder_ok {
        return Err(AppError::Validation(
            "licence holder does not match your account".to_string(),
        ));
    }

    let txn = state.db.begin().await?;
    let existing = profiles::get(&txn, user.id).await?;
    if let (Some(profile_birthdate), Some(licence_birthdate)) = (
        existing.as_ref().and_then(|p| p.birthdate),
        preview.birthdate,
    ) {
        if profile_birthdate != licence_birthdate {
            return Err(AppError::Validation(
                "licence holder does not match your account".to_string(),
            ));
        }
    }

    // 1) Certifications first, so the profile validation below sees the
    //    freshly imported grades. `imported` cannot displace a `validated`
    //    row — that surfaces as a Conflict we translate into "skipped".
    let mut imported: Vec<String> = Vec::new();
    let mut skipped_stronger = Vec::new();
    for brevet in &preview.brevets {
        let Some(code) = &brevet.certification else {
            continue;
        };
        let res = certifications::upsert_row(
            &txn,
            user.id,
            certifications::NewCertification {
                certification: code.clone(),
                obtained_on: brevet.delivered_on,
                provenance: PROVENANCE_IMPORTED,
                recorded_by_user_id: Some(actor.user_id),
                recorded_in_club_id: None,
            },
        )
        .await;
        match res {
            Ok(_) => imported.push(code.clone()),
            Err(AppError::Conflict(_)) => skipped_stronger.push(code.clone()),
            Err(e) => return Err(e),
        }
    }
    // One preparing purge for the whole batch (review D1).
    certifications::clear_preparing_among(&txn, user.id, &imported).await?;

    // 2) Profile fields: patch birthdate + licence number, keep the rest
    //    (the stored CACI included — it stays staff-owned).
    let mut data = profiles::ProfileData::from_existing(existing.as_ref());
    if preview.birthdate.is_some() {
        data.birthdate = preview.birthdate;
    }
    if preview.licence_number.is_some() {
        data.licence_number = preview.licence_number.clone();
    }
    let held = ffessm_certifications(&txn, user.id).await?;
    let statuses = training_statuses::list(&txn, user.id).await?;
    profiles::apply(&txn, user.id, existing, data, &held, &statuses).await?;

    let unmapped = preview.unmapped_labels();
    audit::record(
        &txn,
        actor,
        None,
        "ffessm.licence.imported",
        "user",
        Some(user.id),
        json!({
            "licence_number": preview.licence_number,
            "imported": imported,
            "skipped_stronger": skipped_stronger,
            "unmapped": unmapped,
        }),
    )
    .await?;
    txn.commit().await?;

    Ok(ImportOutcome {
        holder_name: preview.holder_name,
        licence_number: preview.licence_number,
        imported,
        skipped_stronger,
        unmapped,
    })
}

#[cfg(test)]
mod tests {
    use super::holder_matches;

    #[test]
    fn holder_matching_is_accent_and_civility_tolerant() {
        assert!(holder_matches("Mr PRENOM NOM", "Prénom", "Nom"));
        assert!(holder_matches(
            "Mme MARIE-HÉLÈNE DE LA FONTAINE",
            "Marie-Hélène",
            "de la Fontaine"
        ));
        // Another person's licence: refused.
        assert!(!holder_matches("Mr PRENOM NOM", "Autre", "Nom"));
        assert!(!holder_matches("Mr PRENOM NOM", "Prénom", "Autre"));
        // A name that is a strict superset of the account's does match
        // (compound holder names on the licence side).
        assert!(holder_matches("Mr JEAN PIERRE DURAND", "Jean", "Durand"));
    }
}
