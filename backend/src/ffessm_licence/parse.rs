//! Pure parser for the `infolicencie.ffessm.fr` licence page.
//!
//! The page is server-rendered Bootstrap: a stack of `.card` blocks, each
//! with a `.card-header` title and a `.card-body` of `<p>` elements. Values
//! are plain paragraphs; the brevets card alternates a bold title paragraph
//! with an italic `"Délivré à <lieu> le <JJ/MM/AAAA>"` paragraph.
//!
//! This module does NO I/O — it takes an HTML string and returns a
//! [`LicencePreview`]. Unrecognized brevet titles are preserved with a
//! `certification: None` so staff see them instead of losing data.

use chrono::NaiveDate;
use ffessm::Certification;
use scraper::{Html, Selector};
use serde::Serialize;

/// Everything we could read off a licence page, ready to pre-fill a member
/// form. Every field is optional: the page layout is not a contract, so a
/// missing section degrades to `None`/empty rather than failing the lookup.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LicencePreview {
    pub holder_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub licence_number: Option<String>,
    pub licence_type: Option<String>,
    pub subscribed_on: Option<NaiveDate>,
    pub valid_until: Option<NaiveDate>,
    pub club_name: Option<String>,
    pub club_code: Option<String>,
    pub insurance: Option<String>,
    pub insurance_valid_until: Option<NaiveDate>,
    /// `Some(true)` = a valid CACI is on file, `Some(false)` = missing or
    /// invalid, `None` = the section was absent or unreadable.
    pub medical_certificate_valid: Option<bool>,
    pub brevets: Vec<Brevet>,
}

impl LicencePreview {
    /// A page that yielded neither a licence number nor any brevet is almost
    /// certainly an error/redirect page (bad key), not a real licence.
    pub fn looks_empty(&self) -> bool {
        self.licence_number.is_none() && self.brevets.is_empty()
    }

    /// Distinct, recognized certification codes, in page order. This is what
    /// the member form pre-checks.
    pub fn mapped_codes(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.brevets
            .iter()
            .filter_map(|b| b.certification.clone())
            .filter(|code| seen.insert(code.clone()))
            .collect()
    }

    /// Titles we could not map — surfaced to staff (never silently dropped).
    pub fn unmapped_labels(&self) -> Vec<String> {
        self.brevets
            .iter()
            .filter(|b| b.certification.is_none())
            .map(|b| b.label.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Brevet {
    /// Title exactly as printed on the licence.
    pub label: String,
    /// Frozen certification code (`ffessm::Certification`), or `None` when the
    /// title is not recognized.
    pub certification: Option<String>,
    pub delivered_place: Option<String>,
    pub delivered_on: Option<NaiveDate>,
}

/// Parse a licence page into a [`LicencePreview`]. Never fails: an
/// unrecognized layout yields an empty-ish preview (see
/// [`LicencePreview::looks_empty`]).
pub fn parse_licence_page(html: &str) -> LicencePreview {
    use std::sync::LazyLock;
    // Constant selectors, compiled once.
    static CARD: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".card").unwrap());
    static HEADER: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".card-header").unwrap());
    static BODY_P: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".card-body p").unwrap());
    let (card, header_sel, body_p) = (&*CARD, &*HEADER, &*BODY_P);

    let doc = Html::parse_document(html);
    let mut preview = LicencePreview::default();

    for card in doc.select(card) {
        let header = card
            .select(header_sel)
            .next()
            .map(|h| collapse_ws(&text_of(h)))
            .unwrap_or_default();
        let key = fold(&header);

        // (text, is_bold) for each paragraph in this card's body.
        let paras: Vec<(String, bool)> = card
            .select(body_p)
            .map(|p| {
                let classes = p.value().attr("class").unwrap_or_default();
                (
                    collapse_ws(&text_of(p)),
                    classes.contains("font-weight-bold"),
                )
            })
            .filter(|(t, _)| !t.is_empty())
            .collect();

        if key.contains("information sur le plongeur") {
            // Plain paragraphs: name, then "Né(e) le JJ/MM/AAAA".
            if let Some((name, _)) = paras.first() {
                preview.holder_name = Some(name.clone());
            }
            preview.birthdate = paras
                .iter()
                .find(|(t, _)| fold(t).starts_with("ne e le"))
                .and_then(|(t, _)| first_fr_date(t));
        } else if key.contains("information sur la licence") {
            for (label, value) in label_value_pairs(&paras) {
                let l = fold(&label);
                if l.starts_with("licence n") {
                    preview.licence_number = Some(value);
                } else if l.starts_with("type de licence") {
                    preview.licence_type = Some(value);
                } else if l.starts_with("date de souscription") {
                    preview.subscribed_on = parse_fr_date(&value);
                } else if l.starts_with("date de validite") {
                    preview.valid_until = parse_fr_date(&value);
                } else if l.starts_with("structure de plongee") {
                    let (name, code) = split_club(&value);
                    preview.club_name = Some(name);
                    preview.club_code = code;
                }
            }
        } else if key.contains("information sur l assurance") {
            for (label, value) in label_value_pairs(&paras) {
                let l = fold(&label);
                if l.starts_with("assurance") {
                    preview.insurance = Some(value);
                } else if l.starts_with("date de validite") {
                    preview.insurance_valid_until = parse_fr_date(&value);
                }
            }
        } else if key.contains("certificat medical") {
            // The status is the first (bold) paragraph; ❌ / "manquant ou
            // invalide" = invalid, ✅ / "valide" = valid.
            if let Some((status, _)) = paras.first() {
                let s = fold(status);
                preview.medical_certificate_valid =
                    if status.contains('❌') || s.contains("manquant") || s.contains("invalide") {
                        Some(false)
                    } else if status.contains('✅') || s.contains("valide") {
                        Some(true)
                    } else {
                        None
                    };
            }
        } else if key.contains("information sur les brevets") {
            preview.brevets = parse_brevets(&paras);
        }
    }

    preview
}

/// Walk (title, "Délivré à …") paragraph pairs into brevets.
fn parse_brevets(paras: &[(String, bool)]) -> Vec<Brevet> {
    let mut brevets = Vec::new();
    let mut i = 0;
    while i < paras.len() {
        let (label, is_bold) = &paras[i];
        if !is_bold {
            i += 1;
            continue;
        }
        let mut brevet = Brevet {
            certification: Certification::from_ffessm_label(label).map(|c| c.code().to_string()),
            label: label.clone(),
            delivered_place: None,
            delivered_on: None,
        };
        // The following non-bold "Délivré à <lieu> le <date>" paragraph.
        if let Some((detail, false)) = paras.get(i + 1) {
            if fold(detail).starts_with("delivre a") {
                let (date, place) = parse_delivery(detail);
                brevet.delivered_place = place;
                brevet.delivered_on = date;
            }
            i += 2;
        } else {
            i += 1;
        }
        brevets.push(brevet);
    }
    brevets
}

/// Pair each bold paragraph (a label) with the paragraph that follows it (its
/// value). Used for the licence and insurance cards.
fn label_value_pairs(paras: &[(String, bool)]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < paras.len() {
        let (label, is_bold) = &paras[i];
        if *is_bold {
            if let Some((value, _)) = paras.get(i + 1) {
                pairs.push((label.clone(), value.clone()));
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    pairs
}

/// `"UNION SPORTIVE ISSOIRIENNE (14630007)"` → (name, Some(code)).
fn split_club(value: &str) -> (String, Option<String>) {
    if let Some(open) = value.rfind('(') {
        if let Some(close) = value[open..].find(')') {
            let code = value[open + 1..open + close].trim().to_string();
            let name = value[..open].trim().to_string();
            if !code.is_empty() {
                return (name, Some(code));
            }
        }
    }
    (value.trim().to_string(), None)
}

/// `"05/04/2026"` → `NaiveDate`. Anything else → `None`.
fn parse_fr_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%d/%m/%Y").ok()
}

/// First `JJ/MM/AAAA` date anywhere in `text`. Robust to surrounding prose
/// (`"Né(e) le 13/10/1984"`, `"… le 05/04/2026"`) because it splits on any
/// character that is neither a digit nor a slash.
fn first_fr_date(text: &str) -> Option<NaiveDate> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '/'))
        .find_map(parse_fr_date)
}

/// Split a `"Délivré à <lieu> le <JJ/MM/AAAA>"` line into (place, date). The
/// place is the text between the first `à` and the last ` le ` (so place
/// names containing `" le "`/`" la "` survive); the date is scanned
/// independently.
fn parse_delivery(detail: &str) -> (Option<NaiveDate>, Option<String>) {
    let date = first_fr_date(detail);
    let after_a = match detail.char_indices().find(|&(_, c)| c == 'à' || c == 'À') {
        Some((i, c)) => &detail[i + c.len_utf8()..],
        None => detail,
    };
    let lower = after_a.to_lowercase();
    let place = match lower.rfind(" le ") {
        Some(pos) => after_a[..pos].trim(),
        None => after_a.trim(),
    };
    let place = (!place.is_empty()).then(|| place.to_string());
    (date, place)
}

/// Concatenated text content of an element, whitespace-collapsed later.
fn text_of(el: scraper::ElementRef) -> String {
    el.text().collect::<String>()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Lowercase + accent-fold + collapse non-alphanumerics to single spaces,
/// for tolerant matching of section headers and label prefixes.
pub(crate) fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    // Unicode-lowercase FIRST so uppercase accents ("HÉLÈNE") fold too.
    for ch in s.chars().flat_map(char::to_lowercase) {
        let f = match ch {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'û' | 'ü' | 'ú' | 'ù' => 'u',
            'ÿ' => 'y',
            other => other,
        };
        if f.is_ascii_alphanumeric() {
            out.push(f);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anonymized fixture with the EXACT structure of a real
    /// `infolicencie.ffessm.fr` page (synthetic personal data, real brevet
    /// titles including the accented and `E1`/`E2`-prefixed forms).
    const FIXTURE: &str = include_str!("testdata/licence_sample.html");

    #[test]
    fn parses_identity_licence_and_insurance() {
        let p = parse_licence_page(FIXTURE);
        assert_eq!(p.holder_name.as_deref(), Some("Mr PRENOM NOM"));
        assert_eq!(
            p.birthdate,
            Some(NaiveDate::from_ymd_opt(1984, 10, 13).unwrap())
        );
        assert_eq!(p.licence_number.as_deref(), Some("A-00-000000"));
        assert_eq!(p.licence_type.as_deref(), Some("Pratiquant"));
        assert_eq!(
            p.subscribed_on,
            Some(NaiveDate::from_ymd_opt(2025, 9, 8).unwrap())
        );
        assert_eq!(
            p.valid_until,
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap())
        );
        assert_eq!(p.club_name.as_deref(), Some("CLUB DE TEST"));
        assert_eq!(p.club_code.as_deref(), Some("00000000"));
        assert_eq!(p.insurance.as_deref(), Some("AIA : Loisir 3"));
        assert_eq!(
            p.insurance_valid_until,
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap())
        );
        assert_eq!(p.medical_certificate_valid, Some(false));
        assert!(!p.looks_empty());
    }

    #[test]
    fn parses_and_maps_every_brevet_with_place_and_date() {
        let p = parse_licence_page(FIXTURE);
        // Nine brevets, all recognized (no unmapped titles).
        assert_eq!(p.brevets.len(), 9);
        assert!(
            p.unmapped_labels().is_empty(),
            "unexpected unmapped: {:?}",
            p.unmapped_labels()
        );
        assert_eq!(
            p.mapped_codes(),
            vec![
                "N5",
                "N4",
                "INITIATEUR",
                "NITROX_C",
                "N3",
                "RIFAP",
                "N2",
                "NITROX",
                "N1"
            ]
        );
        let first = &p.brevets[0];
        assert_eq!(first.label, "Plongeur Niveau 5");
        assert_eq!(first.certification.as_deref(), Some("N5"));
        assert_eq!(first.delivered_place.as_deref(), Some("GPES La Ciotat"));
        assert_eq!(
            first.delivered_on,
            Some(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap())
        );
    }

    #[test]
    fn unrecognized_brevet_is_kept_as_unmapped() {
        let html = r#"<div class="card"><div class="card-header">Information sur les brevets</div>
            <div class="card-body">
              <p class="font-weight-bold">Niveau 1</p>
              <p class="font-italic">Délivré à Issoire le 10/05/2022</p>
              <p class="font-weight-bold">Stage plongée sous glace</p>
              <p class="font-italic">Délivré à Tignes le 01/02/2023</p>
            </div></div>"#;
        let p = parse_licence_page(html);
        assert_eq!(p.brevets.len(), 2);
        assert_eq!(p.mapped_codes(), vec!["N1"]);
        assert_eq!(p.unmapped_labels(), vec!["Stage plongée sous glace"]);
    }

    #[test]
    fn empty_or_error_page_looks_empty() {
        assert!(parse_licence_page("<html><body>Erreur 404</body></html>").looks_empty());
        assert!(parse_licence_page("").looks_empty());
    }
}
