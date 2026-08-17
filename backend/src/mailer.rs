use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::SmtpConfig;
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct CapturedEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Outgoing mail. Without SMTP configuration the mailer degrades to logging
/// (dev mode) instead of blocking boot — see foundation design §6.
/// (The variant size difference is irrelevant: exactly one Mailer exists.)
#[allow(clippy::large_enum_variant)]
pub enum Mailer {
    Smtp {
        transport: AsyncSmtpTransport<Tokio1Executor>,
        from: Mailbox,
    },
    DevLog,
    /// Test-only: stores emails in memory so tests can read tokens out of
    /// them. Compiled out of production builds.
    #[cfg(test)]
    Capture(std::sync::Mutex<Vec<CapturedEmail>>),
}

impl Mailer {
    pub fn from_config(smtp: Option<&SmtpConfig>) -> Result<Self, String> {
        match smtp {
            None => Ok(Mailer::DevLog),
            Some(cfg) => {
                let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
                    .map_err(|e| format!("invalid smtp host: {e}"))?
                    .port(cfg.port)
                    .credentials(Credentials::new(cfg.username.clone(), cfg.password.clone()))
                    .build();
                let from: Mailbox = format!("{} <{}>", cfg.from_name, cfg.from_email)
                    .parse()
                    .map_err(|e| format!("invalid smtp from address: {e}"))?;
                Ok(Mailer::Smtp { transport, from })
            }
        }
    }

    #[cfg(test)]
    pub fn capture() -> Self {
        Mailer::Capture(std::sync::Mutex::new(Vec::new()))
    }

    /// Test helper: returns captured emails (empty for other variants).
    #[cfg(test)]
    pub fn captured(&self) -> Vec<CapturedEmail> {
        match self {
            Mailer::Capture(mails) => mails.lock().expect("mailer poisoned").clone(),
            _ => Vec::new(),
        }
    }

    pub async fn send(&self, to: &str, subject: &str, html_body: String) -> Result<(), AppError> {
        match self {
            Mailer::DevLog => {
                tracing::info!(to, subject, body = %html_body, "dev mailer: email not sent");
                Ok(())
            }
            #[cfg(test)]
            Mailer::Capture(mails) => {
                mails.lock().expect("mailer poisoned").push(CapturedEmail {
                    to: to.to_string(),
                    subject: subject.to_string(),
                    body: html_body,
                });
                Ok(())
            }
            Mailer::Smtp { transport, from } => {
                let to_mailbox: Mailbox = to.parse().map_err(|_| {
                    AppError::Validation("invalid recipient email address".to_string())
                })?;
                let message = Message::builder()
                    .from(from.clone())
                    .to(to_mailbox)
                    .subject(subject)
                    .header(ContentType::TEXT_HTML)
                    .body(html_body)
                    .map_err(|e| {
                        tracing::error!(error = %e, "failed to build email");
                        AppError::Internal
                    })?;
                transport.send(message).await.map_err(|e| {
                    tracing::error!(error = %e, to, subject, "failed to send email");
                    AppError::Internal
                })?;
                Ok(())
            }
        }
    }
}

/// Minimal HTML escaping for values interpolated into email templates.
/// Fosse interpolated unescaped values into HTML; never repeat that.
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
