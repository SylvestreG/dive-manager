use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub smtp: Option<SmtpConfig>,
    /// Base URL (frontend origin) used to build links in emails.
    pub base_url: String,
    /// Allowed CORS origins (empty = same-origin only).
    pub cors_allowed_origins: Vec<String>,
    /// Origin of the FFESSM licence-info service scraped by the licence
    /// lookup. Overridable so tests can point at a local stub.
    pub ffessm_infolicence_base_url: String,
    /// SSO providers (ADR-0009). A provider absent here is simply not
    /// offered on the login screen.
    pub oidc: OidcConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct OidcConfig {
    pub google: Option<OidcClientConfig>,
    pub microsoft: Option<OidcClientConfig>,
    pub apple: Option<AppleClientConfig>,
}

#[derive(Clone, Deserialize)]
pub struct OidcClientConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Test/dev override of the provider's endpoints (defaults are the real
    /// provider's).
    #[serde(default)]
    pub endpoints: Option<OidcEndpoints>,
}

impl std::fmt::Debug for OidcClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcClientConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .field("endpoints", &self.endpoints)
            .finish()
    }
}

/// Sign in with Apple: the client secret is a short-lived ES256 JWT signed
/// with the `.p8` key — never a static secret.
#[derive(Debug, Clone, Deserialize)]
pub struct AppleClientConfig {
    /// The Services ID (e.g. `fr.divemanager.web`).
    pub client_id: String,
    pub team_id: String,
    pub key_id: String,
    pub private_key_path: String,
    #[serde(default)]
    pub endpoints: Option<OidcEndpoints>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcEndpoints {
    pub authorize_url: String,
    pub token_url: String,
    pub jwks_url: String,
    pub issuer: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Trust the `X-Forwarded-For` header for client IPs (rate limiting).
    /// Enable ONLY behind a reverse proxy that overwrites the header;
    /// otherwise clients can forge IPs to bypass per-IP limits.
    pub trust_proxy: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            trust_proxy: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
    /// Run pending migrations at startup (dev convenience; use the
    /// `migrate` subcommand in production).
    pub migrate_on_boot: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Used only while no global admin exists in the database (ADR-0004).
    pub bootstrap_admin_emails: Vec<String>,
    pub session_idle_days: i64,
    pub session_max_days: i64,
    pub magic_link_hours: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            bootstrap_admin_emails: vec![],
            session_idle_days: 180,
            session_max_days: 365,
            magic_link_hours: 12,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
}

impl Config {
    /// Loads from `DM_CONFIG_PATH` (default `./config.json`) when the file
    /// exists, then applies environment overrides (`DATABASE_URL`,
    /// `DM_BASE_URL`, `DM_SMTP_PASSWORD`).
    pub fn load() -> Result<Self, String> {
        let explicit_path = std::env::var("DM_CONFIG_PATH").ok();
        let path = explicit_path
            .clone()
            .unwrap_or_else(|| "config.json".to_string());
        let mut config: Config = match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map_err(|e| format!("invalid config file {path}: {e}"))?,
            Err(e) if explicit_path.is_some() => {
                // An explicitly requested config file must exist — a typoed
                // path silently running on defaults is a production foot-gun.
                return Err(format!("cannot read config file {path}: {e}"));
            }
            Err(_) => {
                tracing::warn!(path, "no config file found, running on defaults + env");
                Config::default()
            }
        };
        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database.url = url;
        }
        if let Ok(base) = std::env::var("DM_BASE_URL") {
            config.base_url = base;
        }
        if let (Some(smtp), Ok(password)) = (&mut config.smtp, std::env::var("DM_SMTP_PASSWORD")) {
            smtp.password = password;
        }
        if config.base_url.is_empty() {
            config.base_url = format!("http://{}:{}", config.server.host, config.server.port);
        }
        if config.ffessm_infolicence_base_url.is_empty() {
            config.ffessm_infolicence_base_url = "https://infolicencie.ffessm.fr".to_string();
        }
        if config.database.url.is_empty() {
            return Err("database.url is not set (config file or DATABASE_URL)".to_string());
        }
        Ok(config)
    }
}
