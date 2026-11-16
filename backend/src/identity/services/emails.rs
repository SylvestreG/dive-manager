//! Identity email templates. French copy is user-facing UI text and lives
//! here as the embryo of a translation layer; everything interpolated is
//! HTML-escaped.

use crate::mailer::escape_html;

pub fn magic_link_email(first_name: &str, url: &str, valid_hours: i64) -> (String, String) {
    let subject = "Votre lien de connexion Dive Manager".to_string();
    let body = format!(
        "<p>Bonjour {name},</p>\
         <p>Voici votre lien de connexion à Dive Manager&nbsp;:</p>\
         <p><a href=\"{url}\">Se connecter</a></p>\
         <p>Ce lien est valable {hours}&nbsp;heures et ne peut être utilisé qu'une seule fois. \
         Si vous n'êtes pas à l'origine de cette demande, ignorez cet email.</p>",
        name = escape_html(first_name),
        url = escape_html(url),
        hours = valid_hours,
    );
    (subject, body)
}

/// Welcome email after an admin created or added the account (ADR-0009: no
/// invitation token — it points to the login page, where the person picks
/// a login method).
pub fn welcome_email(
    first_name: &str,
    club_name: Option<&str>,
    login_url: &str,
) -> (String, String) {
    let subject = match club_name {
        Some(club) => format!("Bienvenue sur Dive Manager — {club}"),
        None => "Bienvenue sur Dive Manager".to_string(),
    };
    let context = match club_name {
        Some(club) => format!(
            "Votre club {} vous a ajouté sur Dive Manager.",
            escape_html(club)
        ),
        None => "Un administrateur vous a ajouté sur Dive Manager.".to_string(),
    };
    let body = format!(
        "<p>Bonjour {name},</p>\
         <p>{context}</p>\
         <p><a href=\"{url}\">Se connecter</a></p>\
         <p>Pour vous connecter, demandez un lien de connexion par email depuis cette page.</p>",
        name = escape_html(first_name),
        url = escape_html(login_url),
    );
    (subject, body)
}

/// Login page URL with the address pre-filled.
pub fn login_url(base_url: &str, email: &str) -> String {
    let encoded: String = url_encode(email);
    format!("{base_url}/login?email={encoded}")
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
