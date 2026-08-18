//! Outbound fetch of the `infolicencie.ffessm.fr` licence page.
//!
//! The browser cannot fetch this page itself (no CORS, and it is HTML not an
//! API), so the lookup runs server-side: resolve a `{number, key}` target
//! (either passed directly or extracted from the QR-code URL), GET the page,
//! and hand the HTML to [`super::parse`].

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Url;

use crate::errors::AppError;

/// Upstream pages are a few tens of KiB; anything past this is not a licence.
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

/// One shared client: connection pool + TLS session reuse. Redirects are NOT
/// followed (a wrong/expired key answers `302` to an error page, which we map
/// to [`LookupError::NotFound`] instead of scraping it).
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("dive-manager/0.1 (+licence-lookup)")
        .build()
        .expect("static reqwest client")
});

/// The `number` + `key` pair a licence page is addressed by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenceTarget {
    pub number: String,
    pub key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[error("invalid licence reference")]
    InvalidTarget,
    #[error("licence not found or wrong key")]
    NotFound,
    #[error("ffessm service unreachable")]
    Upstream,
}

impl From<LookupError> for AppError {
    /// Single API mapping for every licence endpoint (English messages,
    /// translated client-side like the rest of the backend).
    fn from(err: LookupError) -> AppError {
        match err {
            LookupError::InvalidTarget => {
                AppError::Validation("invalid licence link or number".to_string())
            }
            LookupError::NotFound => {
                AppError::NotFound("licence not found or invalid key".to_string())
            }
            LookupError::Upstream => {
                // Do not leak that the failure is upstream vs internal.
                AppError::Internal
            }
        }
    }
}

impl LicenceTarget {
    /// Build a target from an explicit number/key pair. Both are required and
    /// lightly validated (the key is short and alphanumeric; the number is
    /// digits, with or without the `A-22-` licence prefix).
    pub fn from_number_key(number: &str, key: &str) -> Result<Self, LookupError> {
        let number = normalize_number(number);
        let key = key.trim().to_string();
        if !valid_number(&number) || !valid_key(&key) {
            return Err(LookupError::InvalidTarget);
        }
        Ok(LicenceTarget { number, key })
    }

    /// Extract a target from a scanned QR-code URL such as
    /// `https://infolicencie.ffessm.fr/Home/InfoLicence?number=123456&key=ABCDEF`.
    /// Rejects any host outside `ffessm.fr` (SSRF guard).
    pub fn from_url(raw: &str) -> Result<Self, LookupError> {
        let url = Url::parse(raw.trim()).map_err(|_| LookupError::InvalidTarget)?;
        if url.scheme() != "https" && url.scheme() != "http" {
            return Err(LookupError::InvalidTarget);
        }
        if !host_is_ffessm(&url) {
            return Err(LookupError::InvalidTarget);
        }
        let mut number = None;
        let mut key = None;
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "number" => number = Some(v.into_owned()),
                "key" => key = Some(v.into_owned()),
                _ => {}
            }
        }
        match (number, key) {
            (Some(n), Some(k)) => Self::from_number_key(&n, &k),
            _ => Err(LookupError::InvalidTarget),
        }
    }

    /// Accept either a full URL or a bare number/key pair from the request.
    pub fn resolve(
        url: Option<&str>,
        number: Option<&str>,
        key: Option<&str>,
    ) -> Result<Self, LookupError> {
        if let Some(url) = url.map(str::trim).filter(|s| !s.is_empty()) {
            return Self::from_url(url);
        }
        match (number, key) {
            (Some(n), Some(k)) => Self::from_number_key(n, k),
            _ => Err(LookupError::InvalidTarget),
        }
    }
}

/// Fetch the raw licence HTML. `base_url` is the service origin
/// (`https://infolicencie.ffessm.fr` in production, overridable for tests).
/// The body is capped at [`MAX_BODY_BYTES`] — the upstream is not ours, it
/// must never be able to balloon our memory.
pub async fn fetch_licence_html(
    base_url: &str,
    target: &LicenceTarget,
) -> Result<String, LookupError> {
    let base = Url::parse(base_url).map_err(|err| {
        tracing::warn!(error = %err, "ffessm lookup: invalid base url");
        LookupError::Upstream
    })?;
    let mut endpoint = base.join("/Home/InfoLicence").map_err(|err| {
        tracing::warn!(error = %err, "ffessm lookup: cannot build endpoint");
        LookupError::Upstream
    })?;
    endpoint
        .query_pairs_mut()
        .append_pair("number", &target.number)
        .append_pair("key", &target.key);

    let mut resp = HTTP.get(endpoint).send().await.map_err(|err| {
        // `err` carries no query string (reqwest redacts nothing, but this
        // is a connect/timeout error, not the URL) — safe to log.
        tracing::warn!(error = %err, "ffessm lookup: request failed");
        LookupError::Upstream
    })?;

    let status = resp.status();
    if status.is_redirection() || status == reqwest::StatusCode::NOT_FOUND {
        return Err(LookupError::NotFound);
    }
    if !status.is_success() {
        tracing::warn!(%status, "ffessm lookup: unexpected upstream status");
        return Err(LookupError::Upstream);
    }

    let mut body: Vec<u8> = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                if body.len() + chunk.len() > MAX_BODY_BYTES {
                    tracing::warn!("ffessm lookup: response exceeds body cap");
                    return Err(LookupError::Upstream);
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(err) => {
                tracing::warn!(error = %err, "ffessm lookup: body read failed");
                return Err(LookupError::Upstream);
            }
        }
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// Strip the `A-22-` style prefix and any separators, leaving the digits the
/// service expects in `?number=`.
fn normalize_number(raw: &str) -> String {
    let trimmed = raw.trim();
    // Keep only the trailing numeric run (handles "A-22-123456" → "123456"
    // and a bare "123456").
    match trimmed.rsplit(['-', ' ']).next() {
        Some(tail) if tail.chars().all(|c| c.is_ascii_digit()) && !tail.is_empty() => {
            tail.to_string()
        }
        _ => trimmed.chars().filter(|c| c.is_ascii_digit()).collect(),
    }
}

fn valid_number(n: &str) -> bool {
    !n.is_empty() && n.len() <= 12 && n.chars().all(|c| c.is_ascii_digit())
}

fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.len() <= 32 && k.chars().all(|c| c.is_ascii_alphanumeric())
}

fn host_is_ffessm(url: &Url) -> bool {
    matches!(url.host_str(), Some(h) if h == "ffessm.fr" || h.ends_with(".ffessm.fr"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_number_and_key_from_qr_url() {
        let t = LicenceTarget::from_url(
            "https://infolicencie.ffessm.fr/Home/InfoLicence?number=123456&key=ABCDEF",
        )
        .unwrap();
        assert_eq!(t.number, "123456");
        assert_eq!(t.key, "ABCDEF");
    }

    #[test]
    fn strips_licence_prefix_from_number() {
        let t = LicenceTarget::from_number_key("A-22-123456", "ABCDEF").unwrap();
        assert_eq!(t.number, "123456");
    }

    #[test]
    fn rejects_non_ffessm_hosts() {
        for url in [
            "https://evil.example.com/Home/InfoLicence?number=1&key=x",
            "https://infolicencie.ffessm.fr.evil.com/x?number=1&key=x",
            "file:///etc/passwd",
            "http://169.254.169.254/latest/meta-data?number=1&key=x",
        ] {
            assert!(
                matches!(
                    LicenceTarget::from_url(url),
                    Err(LookupError::InvalidTarget)
                ),
                "should reject {url}"
            );
        }
    }

    #[test]
    fn rejects_missing_or_malformed_params() {
        assert!(LicenceTarget::from_url(
            "https://infolicencie.ffessm.fr/Home/InfoLicence?number=1"
        )
        .is_err());
        assert!(LicenceTarget::from_number_key("", "KEY").is_err());
        assert!(LicenceTarget::from_number_key("123456", "bad key!").is_err());
    }

    /// Live end-to-end fetch, gated on `DM_TEST_LICENCE_URL` so no real
    /// licence reference is ever committed. Run with:
    /// `DM_TEST_LICENCE_URL='https://infolicencie.ffessm.fr/...' \
    ///   cargo test -p dive-manager-backend live_ffessm_lookup -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "hits the live FFESSM service; needs DM_TEST_LICENCE_URL"]
    async fn live_ffessm_lookup() {
        let Ok(url) = std::env::var("DM_TEST_LICENCE_URL") else {
            eprintln!("set DM_TEST_LICENCE_URL to run this");
            return;
        };
        let target = LicenceTarget::from_url(&url).expect("valid url");
        let html = fetch_licence_html("https://infolicencie.ffessm.fr", &target)
            .await
            .expect("fetch");
        let preview = super::super::parse::parse_licence_page(&html);
        eprintln!(
            "licence {:?} | mapped {:?} | unmapped {:?}",
            preview.licence_number,
            preview.mapped_codes(),
            preview.unmapped_labels()
        );
        assert!(!preview.looks_empty(), "live page parsed empty");
    }

    #[test]
    fn resolve_prefers_url_then_falls_back_to_pair() {
        let via_url = LicenceTarget::resolve(
            Some("https://infolicencie.ffessm.fr/x?number=1&key=AB"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(via_url.number, "1");
        let via_pair = LicenceTarget::resolve(None, Some("2"), Some("CD")).unwrap();
        assert_eq!(via_pair.number, "2");
        assert!(LicenceTarget::resolve(None, None, None).is_err());
    }
}
