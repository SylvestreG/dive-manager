//! PDF handling for level documents (lopdf). The stamps are written INTO the
//! page content stream — flattened, printable everywhere, not removable —
//! unlike Fosse's annotations, which any PDF editor could delete.

use std::collections::BTreeMap;

use lopdf::{dictionary, Document, Object, ObjectId, Stream};

use crate::errors::AppError;

/// Resource name of the font we register on stamped pages.
const FONT_KEY: &str = "DMCF";

/// Parses an uploaded template and returns its page count (422 on anything
/// lopdf cannot read).
pub fn page_count(bytes: &[u8]) -> Result<i32, AppError> {
    let document = Document::load_mem(bytes)
        .map_err(|_| AppError::Validation("document must be a valid PDF".to_string()))?;
    let pages = document.get_pages().len();
    if pages == 0 {
        return Err(AppError::Validation("PDF has no pages".to_string()));
    }
    Ok(pages as i32)
}

/// One stamp box with its rendered text lines (top line first).
pub struct Stamp {
    /// 1-based page number.
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub lines: Vec<String>,
}

/// Overlays the stamps on the stored template. The template was validated at
/// upload, so parse failures here are internal errors, not user input.
/// Boxes pointing past the last page (template re-uploaded with fewer pages)
/// are skipped with a warning.
pub fn fill(template: &[u8], stamps: &[Stamp]) -> Result<Vec<u8>, AppError> {
    let mut document = Document::load_mem(template).map_err(|err| {
        tracing::error!(error = %err, "stored level document no longer parses");
        AppError::Internal
    })?;
    let pages = document.get_pages();

    let font_id = document.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Oblique",
        "Encoding" => "WinAnsiEncoding",
    }));

    let mut by_page: BTreeMap<u32, Vec<&Stamp>> = BTreeMap::new();
    for stamp in stamps {
        by_page.entry(stamp.page).or_default().push(stamp);
    }

    for (page_number, group) in by_page {
        let Some(&page_id) = pages.get(&page_number) else {
            tracing::warn!(
                page = page_number,
                "stamp position beyond the last page, skipped"
            );
            continue;
        };
        ensure_font(&mut document, page_id, font_id)?;
        // Sandwich the existing content between "q" and "Q" so an unbalanced
        // graphics state in the template cannot displace our overlay.
        let prefix_id =
            document.add_object(Object::Stream(Stream::new(dictionary! {}, b"q\n".to_vec())));
        let mut content = b"Q\n".to_vec();
        content.extend_from_slice(&render_content(&group));
        let overlay_id = document.add_object(Object::Stream(Stream::new(dictionary! {}, content)));
        wrap_content(&mut document, page_id, prefix_id, overlay_id)?;
    }

    let mut out = Vec::new();
    document.save_to(&mut out).map_err(|err| {
        tracing::error!(error = %err, "failed to serialize filled document");
        AppError::Internal
    })?;
    Ok(out)
}

fn internal(err: lopdf::Error) -> AppError {
    tracing::error!(error = %err, "unexpected PDF structure in stored template");
    AppError::Internal
}

/// Registers our font under `FONT_KEY` in the page's resources, whatever
/// their storage shape (missing / inline / referenced; Font subdictionary
/// inline or referenced).
fn ensure_font(
    document: &mut Document,
    page_id: ObjectId,
    font_id: ObjectId,
) -> Result<(), AppError> {
    enum Location {
        Missing,
        PageInline,
        Referenced(ObjectId),
    }
    let resources_location = {
        let page = document.get_dictionary(page_id).map_err(internal)?;
        match page.get(b"Resources") {
            Ok(Object::Reference(rid)) => Location::Referenced(*rid),
            Ok(Object::Dictionary(_)) => Location::PageInline,
            _ => Location::Missing,
        }
    };

    let resources = match resources_location {
        Location::Missing => {
            let page = document.get_dictionary_mut(page_id).map_err(internal)?;
            page.set("Resources", Object::Dictionary(dictionary! {}));
            match page.get_mut(b"Resources") {
                Ok(Object::Dictionary(dict)) => dict,
                _ => return Err(AppError::Internal),
            }
        }
        Location::PageInline => {
            let page = document.get_dictionary_mut(page_id).map_err(internal)?;
            match page.get_mut(b"Resources") {
                Ok(Object::Dictionary(dict)) => dict,
                _ => return Err(AppError::Internal),
            }
        }
        Location::Referenced(rid) => document.get_dictionary_mut(rid).map_err(internal)?,
    };

    // The Font entry itself may be a reference; resolve it the same way.
    let font_reference = match resources.get(b"Font") {
        Ok(Object::Reference(rid)) => Some(*rid),
        Ok(Object::Dictionary(_)) => None,
        _ => {
            resources.set("Font", Object::Dictionary(dictionary! {}));
            None
        }
    };
    match font_reference {
        Some(rid) => {
            document
                .get_dictionary_mut(rid)
                .map_err(internal)?
                .set(FONT_KEY, Object::Reference(font_id));
        }
        None => {
            match resources.get_mut(b"Font") {
                Ok(Object::Dictionary(fonts)) => {
                    fonts.set(FONT_KEY, Object::Reference(font_id));
                }
                _ => return Err(AppError::Internal),
            };
        }
    }
    Ok(())
}

/// Rewrites the page's Contents as [prefix, ...existing, overlay] (single
/// ref or array), wrapping the original stream(s).
fn wrap_content(
    document: &mut Document,
    page_id: ObjectId,
    prefix_id: ObjectId,
    overlay_id: ObjectId,
) -> Result<(), AppError> {
    let page = document.get_dictionary_mut(page_id).map_err(internal)?;
    let mut array = vec![Object::Reference(prefix_id)];
    match page.get(b"Contents") {
        Ok(Object::Array(existing)) => array.extend(existing.clone()),
        Ok(existing @ Object::Reference(_)) => array.push(existing.clone()),
        _ => {}
    }
    array.push(Object::Reference(overlay_id));
    page.set("Contents", Object::Array(array));
    Ok(())
}

/// Builds the overlay operations for one page: a light rounded-off box per
/// stamp plus its text lines, wrapped in q/Q to isolate graphics state.
fn render_content(stamps: &[&Stamp]) -> Vec<u8> {
    let mut ops: Vec<u8> = Vec::new();
    ops.extend_from_slice(b"q\n");
    for stamp in stamps {
        // Background and border.
        ops.extend_from_slice(
            format!(
                "0.97 0.97 0.97 rg\n{x:.2} {y:.2} {w:.2} {h:.2} re\nf\n\
                 0.55 0.55 0.55 RG\n0.6 w\n{x:.2} {y:.2} {w:.2} {h:.2} re\nS\n",
                x = stamp.x,
                y = stamp.y,
                w = stamp.width,
                h = stamp.height,
            )
            .as_bytes(),
        );
        // Text lines, top-down inside the box.
        let leading = stamp.font_size * 1.2;
        let mut baseline = stamp.y + stamp.height - stamp.font_size - 1.0;
        if stamp.lines.len() == 1 {
            baseline = stamp.y + (stamp.height - stamp.font_size) / 2.0 + 1.5;
        }
        for line in &stamp.lines {
            ops.extend_from_slice(
                format!(
                    "BT\n/{FONT_KEY} {size:.2} Tf\n0.12 0.12 0.12 rg\n{tx:.2} {ty:.2} Td\n(",
                    size = stamp.font_size,
                    tx = stamp.x + 2.0,
                    ty = baseline,
                )
                .as_bytes(),
            );
            for byte in winansi(line) {
                match byte {
                    b'(' | b')' | b'\\' => {
                        ops.push(b'\\');
                        ops.push(byte);
                    }
                    _ => ops.push(byte),
                }
            }
            ops.extend_from_slice(b") Tj\nET\n");
            baseline -= leading;
        }
    }
    ops.extend_from_slice(b"Q\n");
    ops
}

/// UTF-8 → WinAnsi (CP-1252). Latin-1 passes through, the CP-1252 specials
/// (œ, quotes, dashes) are mapped, anything else degrades to '?'.
fn winansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c {
            '€' => 0x80,
            '‚' => 0x82,
            'ƒ' => 0x83,
            '„' => 0x84,
            '…' => 0x85,
            '†' => 0x86,
            '‡' => 0x87,
            'ˆ' => 0x88,
            '‰' => 0x89,
            'Š' => 0x8A,
            '‹' => 0x8B,
            'Œ' => 0x8C,
            'Ž' => 0x8E,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '˜' => 0x98,
            '™' => 0x99,
            'š' => 0x9A,
            '›' => 0x9B,
            'œ' => 0x9C,
            'ž' => 0x9E,
            'Ÿ' => 0x9F,
            c if (c as u32) < 0x80 => c as u8,
            c if (0xA0..=0xFF).contains(&(c as u32)) => c as u32 as u8,
            _ => b'?',
        })
        .collect()
}

/// Minimal one-page PDF for tests (unit + integration).
#[cfg(test)]
pub fn tiny_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![Object::Reference(page_id)],
        "Count" => 1,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("save tiny pdf");
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_count_parses_and_rejects() {
        assert_eq!(page_count(&tiny_pdf()).expect("count"), 1);
        assert!(page_count(b"not a pdf at all").is_err());
    }

    #[test]
    fn fill_produces_a_parsable_pdf_with_the_overlay() {
        let filled = fill(
            &tiny_pdf(),
            &[Stamp {
                page: 1,
                x: 100.0,
                y: 700.0,
                width: 120.0,
                height: 24.0,
                font_size: 8.0,
                lines: vec!["14/03/2026".to_string(), "Léa Cœur (E3)".to_string()],
            }],
        )
        .expect("fill");
        let reparsed = Document::load_mem(&filled).expect("filled PDF parses");
        assert_eq!(reparsed.get_pages().len(), 1);
        // The original content is sandwiched between q/Q so an unbalanced
        // template state cannot displace the overlay.
        let page_id = *reparsed.get_pages().get(&1).expect("page 1");
        let content = reparsed.get_page_content(page_id).expect("content");
        let text = String::from_utf8_lossy(&content);
        assert!(text.starts_with("q\n"), "{text}");
        assert!(text.contains("Q\nq\n"), "{text}");
    }

    #[test]
    fn out_of_range_pages_are_skipped_not_fatal() {
        let filled = fill(
            &tiny_pdf(),
            &[Stamp {
                page: 9,
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
                font_size: 8.0,
                lines: vec!["x".to_string()],
            }],
        )
        .expect("fill");
        assert!(Document::load_mem(&filled).is_ok());
    }

    #[test]
    fn winansi_maps_french_and_degrades_the_rest() {
        assert_eq!(winansi("Léa"), vec![b'L', 0xE9, b'a']);
        assert_eq!(winansi("Cœur"), vec![b'C', 0x9C, b'u', b'r']);
        assert_eq!(winansi("日本"), vec![b'?', b'?']);
    }
}
