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
