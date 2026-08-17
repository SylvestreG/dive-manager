//! Tenancy email templates (invitations). Interpolations are HTML-escaped.

use crate::mailer::escape_html;

pub fn invitation_email(
    first_name: &str,
    inviter_name: &str,
    club_name: Option<&str>,
    url: &str,
    valid_days: i64,
) -> (String, String) {
    let (subject, target) = match club_name {
        Some(club) => (
            format!("Invitation à rejoindre {club} sur Dive Manager"),
            format!("le club <strong>{}</strong>", escape_html(club)),
        ),
        None => (
            "Invitation à administrer Dive Manager".to_string(),
            "Dive Manager en tant qu'administrateur".to_string(),
        ),
    };
    let body = format!(
        "<p>Bonjour {name},</p>\
         <p>{inviter} vous invite à rejoindre {target}.</p>\
         <p><a href=\"{url}\">Accepter l'invitation</a></p>\
         <p>Cette invitation est valable {days}&nbsp;jours.</p>",
        name = escape_html(first_name),
        inviter = escape_html(inviter_name),
        target = target,
        url = escape_html(url),
        days = valid_days,
    );
    (subject, body)
}
