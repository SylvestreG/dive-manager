//! OIDC login and identity linking (ADR-0009, auth-onboarding design §3.2,
//! §11): authorization-code flow with PKCE (S256), nonce and state; the
//! id_token is verified LOCALLY against the provider's JWKS (iss, aud, exp,
//! nonce) — no tokeninfo round-trip.
//!
//! Login resolution (R8), in order:
//! 1. known `(provider, subject)` on an active user → session;
//! 2. (slice 3: pending access request);
//! 3. TRUSTED provider email equal to an active user's email → link + session;
//! 4. untrusted email equal to an account's email → confirmation magic link;
//! 5. otherwise → "unknown account" (slice 3 turns it into a registration).

use std::collections::HashMap;
use std::time::{Duration as StdDuration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Duration;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::{Config, OidcEndpoints};
use crate::errors::AppError;
use crate::identity::entities::{
    exchange_codes, login_identities, login_magic_links, oidc_flows, prelude::*, users,
};
use crate::identity::services::audit::{self, Actor};
use crate::identity::services::{emails, sessions, tokens, users as user_service};
use crate::{now, AppState};

/// Microsoft's tenant id for personal accounts (outlook.com, hotmail…):
/// only there is the `email` claim ownership-proven (design R7).
pub const MICROSOFT_CONSUMER_TENANT: &str = "9188040d-6c67-4c5b-b112-36a304b66dad";
const FLOW_TTL_MINUTES: i64 = 10;
const EXCHANGE_TTL_SECONDS: i64 = 60;
const JWKS_TTL: StdDuration = StdDuration::from_secs(3600);
const JWKS_MIN_REFRESH: StdDuration = StdDuration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Google,
    Microsoft,
    Apple,
}

impl Provider {
    pub const ALL: [Provider; 3] = [Provider::Google, Provider::Microsoft, Provider::Apple];

    pub fn parse(value: &str) -> Option<Provider> {
        match value {
            "google" => Some(Provider::Google),
            "microsoft" => Some(Provider::Microsoft),
            "apple" => Some(Provider::Apple),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Microsoft => "microsoft",
            Provider::Apple => "apple",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Google => "Google",
            Provider::Microsoft => "Microsoft",
            Provider::Apple => "Apple",
        }
    }

    fn default_endpoints(self) -> OidcEndpoints {
        let (authorize_url, token_url, jwks_url, issuer) = match self {
            Provider::Google => (
                "https://accounts.google.com/o/oauth2/v2/auth",
                "https://oauth2.googleapis.com/token",
                "https://www.googleapis.com/oauth2/v3/certs",
                "https://accounts.google.com",
            ),
            Provider::Microsoft => (
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
                "https://login.microsoftonline.com/common/oauth2/v2.0/token",
                "https://login.microsoftonline.com/common/discovery/v2.0/keys",
                // `common` endpoint: the issuer is per tenant (design §11).
                "https://login.microsoftonline.com/{tid}/v2.0",
            ),
            Provider::Apple => (
                "https://appleid.apple.com/auth/authorize",
                "https://appleid.apple.com/auth/token",
                "https://appleid.apple.com/auth/keys",
                "https://appleid.apple.com",
            ),
        };
        OidcEndpoints {
            authorize_url: authorize_url.to_string(),
            token_url: token_url.to_string(),
            jwks_url: jwks_url.to_string(),
            issuer: issuer.to_string(),
        }
    }
}

/// Providers configured on this instance (login buttons).
pub fn configured(config: &Config) -> Vec<Provider> {
    Provider::ALL
        .into_iter()
        .filter(|p| match p {
            Provider::Google => config.oidc.google.is_some(),
            Provider::Microsoft => config.oidc.microsoft.is_some(),
            Provider::Apple => config.oidc.apple.is_some(),
        })
        .collect()
}

enum ClientSecret {
    Static(String),
    Apple {
        team_id: String,
        key_id: String,
        private_key_path: String,
    },
}

struct ProviderSpec {
    provider: Provider,
    client_id: String,
    secret: ClientSecret,
    endpoints: OidcEndpoints,
}

fn spec(config: &Config, provider: Provider) -> Result<ProviderSpec, AppError> {
    let not_configured = || AppError::NotFound("unknown login provider".to_string());
    Ok(match provider {
        Provider::Google | Provider::Microsoft => {
            let client = match provider {
                Provider::Google => config.oidc.google.as_ref(),
                _ => config.oidc.microsoft.as_ref(),
            }
            .ok_or_else(not_configured)?;
            ProviderSpec {
                provider,
                client_id: client.client_id.clone(),
                secret: ClientSecret::Static(client.client_secret.clone()),
                endpoints: client
                    .endpoints
                    .clone()
                    .unwrap_or_else(|| provider.default_endpoints()),
            }
        }
        Provider::Apple => {
            let apple = config.oidc.apple.as_ref().ok_or_else(not_configured)?;
            ProviderSpec {
                provider,
                client_id: apple.client_id.clone(),
                secret: ClientSecret::Apple {
                    team_id: apple.team_id.clone(),
                    key_id: apple.key_id.clone(),
                    private_key_path: apple.private_key_path.clone(),
                },
                endpoints: apple
                    .endpoints
                    .clone()
                    .unwrap_or_else(|| provider.default_endpoints()),
            }
        }
    })
}

/// Callback URL registered at the provider (behind the /v2 prefix in prod).
pub fn redirect_uri(config: &Config, provider: Provider) -> String {
    format!(
        "{}/api/v1/auth/oidc/{}/callback",
        config.base_url,
        provider.as_str()
    )
}

/// Process-wide OIDC runtime: HTTP client + JWKS cache (single instance —
/// no shared cache needed).
pub struct OidcRuntime {
    http: reqwest::Client,
    jwks: RwLock<HashMap<String, (JwkSet, Instant)>>,
}

impl Default for OidcRuntime {
    fn default() -> Self {
        OidcRuntime {
            http: reqwest::Client::builder()
                .timeout(StdDuration::from_secs(10))
                .build()
                .expect("http client"),
            jwks: RwLock::new(HashMap::new()),
        }
    }
}

impl OidcRuntime {
    async fn fetch_jwks(&self, url: &str) -> Result<JwkSet, AppError> {
        let set = self
            .http
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| {
                tracing::error!(error = %e, url, "JWKS fetch failed");
                AppError::Internal
            })?
            .json::<JwkSet>()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, url, "JWKS unparseable");
                AppError::Internal
            })?;
        self.jwks
            .write()
            .await
            .insert(url.to_string(), (set.clone(), Instant::now()));
        Ok(set)
    }

    /// Key for `kid`, refreshing the cached set when stale or when the kid is
    /// unknown (key rotation) — at most once per minute.
    async fn key_for(&self, url: &str, kid: &str) -> Result<DecodingKey, AppError> {
        let cached = self.jwks.read().await.get(url).cloned();
        let set = match cached {
            Some((set, at)) if at.elapsed() < JWKS_TTL && set.find(kid).is_some() => set,
            Some((set, at)) if at.elapsed() < JWKS_MIN_REFRESH => set,
            _ => self.fetch_jwks(url).await?,
        };
        let jwk = set
            .find(kid)
            .ok_or_else(|| invalid_token("unknown signing key"))?;
        DecodingKey::from_jwk(jwk).map_err(|_| invalid_token("unusable signing key"))
    }
}

fn invalid_token(reason: &str) -> AppError {
    tracing::warn!(reason, "OIDC id_token rejected");
    AppError::Unauthorized("invalid identity token".to_string())
}

fn random_b64(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Starts a flow and returns the provider authorization URL. `link_user`
/// binds a `link` flow to the logged-in user who started it.
pub async fn start(
    state: &AppState,
    provider: Provider,
    link_user: Option<Uuid>,
) -> Result<String, AppError> {
    let spec = spec(&state.config, provider)?;
    let state_token = tokens::generate();
    let nonce = random_b64(24);
    let verifier = random_b64(32);
    let created = now();
    oidc_flows::ActiveModel {
        id: Set(Uuid::now_v7()),
        state_hash: Set(state_token.hash),
        provider: Set(provider.as_str().to_string()),
        client: Set(oidc_flows::CLIENT_WEB.to_string()),
        purpose: Set(match link_user {
            Some(_) => oidc_flows::PURPOSE_LINK,
            None => oidc_flows::PURPOSE_LOGIN,
        }
        .to_string()),
        nonce: Set(nonce.clone()),
        pkce_verifier: Set(verifier.clone()),
        link_user_id: Set(link_user),
        consumed_at: Set(None),
        expires_at: Set(created + Duration::minutes(FLOW_TTL_MINUTES)),
        created_at: Set(created),
    }
    .insert(&state.db)
    .await?;

    let redirect = redirect_uri(&state.config, provider);
    let challenge = pkce_challenge(&verifier);
    let mut params: Vec<(&str, &str)> = vec![
        ("response_type", "code"),
        ("client_id", &spec.client_id),
        ("redirect_uri", &redirect),
        ("state", &state_token.token),
        ("nonce", &nonce),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
    ];
    match provider {
        // Apple returns the name/email scopes only with form_post.
        Provider::Apple => {
            params.push(("scope", "name email"));
            params.push(("response_mode", "form_post"));
        }
        Provider::Google | Provider::Microsoft => {
            params.push(("scope", "openid email profile"));
            params.push(("prompt", "select_account"));
        }
    }
    let url =
        reqwest::Url::parse_with_params(&spec.endpoints.authorize_url, &params).map_err(|e| {
            tracing::error!(error = %e, "invalid authorize_url");
            AppError::Internal
        })?;
    Ok(url.to_string())
}

/// Verified identity asserted by the provider.
#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub provider: Provider,
    pub subject: String,
    pub email: Option<String>,
    /// Provider-proven ownership of `email` (design R7).
    pub email_trusted: bool,
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: String,
    iss: String,
    nonce: Option<String>,
    email: Option<String>,
    /// Google: bool; Apple: bool or "true"/"false".
    email_verified: Option<serde_json::Value>,
    /// Microsoft tenant id.
    tid: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
}

#[derive(Serialize)]
struct AppleSecretClaims<'a> {
    iss: &'a str,
    iat: i64,
    exp: i64,
    aud: &'a str,
    sub: &'a str,
}

async fn client_secret(spec: &ProviderSpec) -> Result<String, AppError> {
    match &spec.secret {
        ClientSecret::Static(secret) => Ok(secret.clone()),
        ClientSecret::Apple {
            team_id,
            key_id,
            private_key_path,
        } => {
            let pem = tokio::fs::read(private_key_path).await.map_err(|e| {
                tracing::error!(error = %e, "cannot read Apple private key");
                AppError::Internal
            })?;
            let key = EncodingKey::from_ec_der(&pkcs8_der_from_pem(&pem)?);
            let mut header = Header::new(Algorithm::ES256);
            header.kid = Some(key_id.clone());
            let iat = chrono::Utc::now().timestamp();
            jsonwebtoken::encode(
                &header,
                &AppleSecretClaims {
                    iss: team_id,
                    iat,
                    exp: iat + 300,
                    aud: "https://appleid.apple.com",
                    sub: &spec.client_id,
                },
                &key,
            )
            .map_err(|e| {
                tracing::error!(error = %e, "cannot sign Apple client secret");
                AppError::Internal
            })
        }
    }
}

/// Decodes a PKCS#8 PEM (`-----BEGIN PRIVATE KEY-----`, Apple's `.p8`)
/// into DER.
fn pkcs8_der_from_pem(pem: &[u8]) -> Result<Vec<u8>, AppError> {
    let text = std::str::from_utf8(pem).map_err(|_| AppError::Internal)?;
    let body: String = text
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .map(str::trim)
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|e| {
            tracing::error!(error = %e, "invalid Apple private key (PEM)");
            AppError::Internal
        })
}

/// Consumes the flow for `state` (single use, unexpired, same provider).
async fn take_flow(
    state: &AppState,
    provider: Provider,
    state_param: &str,
) -> Result<oidc_flows::Model, AppError> {
    let invalid = || AppError::Unauthorized("invalid or expired login attempt".to_string());
    let flow = OidcFlows::find()
        .filter(oidc_flows::Column::StateHash.eq(tokens::hash(state_param)))
        .filter(oidc_flows::Column::Provider.eq(provider.as_str()))
        .filter(oidc_flows::Column::ConsumedAt.is_null())
        .filter(oidc_flows::Column::ExpiresAt.gt(now()))
        .one(&state.db)
        .await?
        .ok_or_else(invalid)?;
    let consumed = OidcFlows::update_many()
        .col_expr(oidc_flows::Column::ConsumedAt, Expr::value(Some(now())))
        .filter(oidc_flows::Column::Id.eq(flow.id))
        .filter(oidc_flows::Column::ConsumedAt.is_null())
        .exec(&state.db)
        .await?;
    if consumed.rows_affected != 1 {
        return Err(invalid());
    }
    Ok(flow)
}

/// Exchanges the code and verifies the id_token locally.
async fn verify_code(
    state: &AppState,
    spec: &ProviderSpec,
    flow: &oidc_flows::Model,
    code: &str,
) -> Result<VerifiedIdentity, AppError> {
    let secret = client_secret(spec).await?;
    let redirect = redirect_uri(&state.config, spec.provider);
    let response = state
        .oidc
        .http
        .post(&spec.endpoints.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &redirect),
            ("client_id", &spec.client_id),
            ("client_secret", &secret),
            ("code_verifier", &flow.pkce_verifier),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, provider = spec.provider.as_str(), "token request failed");
            AppError::Internal
        })?;
    if !response.status().is_success() {
        tracing::warn!(
            status = %response.status(),
            provider = spec.provider.as_str(),
            "token endpoint refused the code"
        );
        return Err(AppError::Unauthorized(
            "invalid or expired login attempt".to_string(),
        ));
    }
    let tokens: TokenResponse = response.json().await.map_err(|e| {
        tracing::error!(error = %e, "token response unparseable");
        AppError::Internal
    })?;

    let header = jsonwebtoken::decode_header(&tokens.id_token)
        .map_err(|_| invalid_token("malformed header"))?;
    if !matches!(header.alg, Algorithm::RS256 | Algorithm::ES256) {
        return Err(invalid_token("unexpected algorithm"));
    }
    let kid = header.kid.ok_or_else(|| invalid_token("missing kid"))?;
    let key = state.oidc.key_for(&spec.endpoints.jwks_url, &kid).await?;
    let mut validation = Validation::new(header.alg);
    validation.set_audience(&[&spec.client_id]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    // The issuer may be tenant-templated (Microsoft `common`): checked below.
    validation.iss = None;
    let claims = jsonwebtoken::decode::<IdTokenClaims>(&tokens.id_token, &key, &validation)
        .map_err(|e| invalid_token(&format!("{e}")))?
        .claims;

    let expected_issuer = if spec.endpoints.issuer.contains("{tid}") {
        let tid = claims
            .tid
            .as_deref()
            .ok_or_else(|| invalid_token("missing tid"))?;
        spec.endpoints.issuer.replace("{tid}", tid)
    } else {
        spec.endpoints.issuer.clone()
    };
    let issuer_ok = claims.iss == expected_issuer
        // Google also issues the scheme-less form.
        || (spec.provider == Provider::Google && claims.iss == "accounts.google.com");
    if !issuer_ok {
        return Err(invalid_token("issuer mismatch"));
    }
    if claims.nonce.as_deref() != Some(flow.nonce.as_str()) {
        return Err(invalid_token("nonce mismatch"));
    }

    let email = claims
        .email
        .as_deref()
        .map(crate::normalize_email)
        .filter(|e| !e.is_empty());
    let verified_flag = matches!(claims.email_verified, Some(serde_json::Value::Bool(true)))
        || matches!(
            claims.email_verified.as_ref().and_then(|v| v.as_str()),
            Some("true")
        );
    let email_trusted = email.is_some()
        && match spec.provider {
            Provider::Google | Provider::Apple => verified_flag,
            Provider::Microsoft => claims.tid.as_deref() == Some(MICROSOFT_CONSUMER_TENANT),
        };
    Ok(VerifiedIdentity {
        provider: spec.provider,
        subject: claims.sub,
        email,
        email_trusted,
    })
}

/// Where the browser goes after the callback (frontend routes).
#[derive(Debug, PartialEq, Eq)]
pub enum CallbackOutcome {
    /// Logged in: the page trades this one-time code for a session.
    LoggedIn { exchange_code: String },
    /// An account uses this email but the provider does not prove it: a
    /// confirmation link was emailed to the account address.
    ConfirmationSent,
    /// No account for this identity (slice 3: access request).
    UnknownAccount,
    /// Link flow: identity added to the logged-in user's account.
    Linked,
    /// Link flow: this identity already belongs to another account.
    AlreadyUsed,
    /// Account disabled, or the attempt was invalid.
    Failed,
}

/// Handles the provider callback end to end.
pub async fn callback(
    state: &AppState,
    provider: Provider,
    code: &str,
    state_param: &str,
    client_label: Option<String>,
) -> Result<CallbackOutcome, AppError> {
    let spec = spec(&state.config, provider)?;
    let flow = take_flow(state, provider, state_param).await?;
    let identity = verify_code(state, &spec, &flow, code).await?;

    if flow.purpose == oidc_flows::PURPOSE_LINK {
        let user_id = flow.link_user_id.ok_or(AppError::Internal)?;
        return link_to_user(state, user_id, &identity).await;
    }

    let txn = state.db.begin().await?;
    // 1. Known identity.
    if let Some(linked) = find_identity(&txn, provider, &identity.subject).await? {
        let user = user_service::lock_for_update(&txn, linked.user_id).await?;
        if user.status != users::STATUS_ACTIVE {
            return Ok(CallbackOutcome::Failed);
        }
        touch_identity(&txn, linked).await?;
        let code = issue_exchange_code(&txn, user.id, client_label).await?;
        txn.commit().await?;
        return Ok(CallbackOutcome::LoggedIn {
            exchange_code: code,
        });
    }

    let Some(email) = identity.email.clone() else {
        return Ok(CallbackOutcome::UnknownAccount);
    };
    let Some(user) = user_service::find_by_email(&txn, &email).await? else {
        return Ok(CallbackOutcome::UnknownAccount);
    };
    if user.status != users::STATUS_ACTIVE {
        return Ok(CallbackOutcome::Failed);
    }

    if identity.email_trusted {
        // 3. Trusted email: automatic first link.
        let user = user_service::lock_for_update(&txn, user.id).await?;
        insert_identity(&txn, user.id, &identity).await?;
        user_service::record_login(&txn, user.clone(), true).await?;
        audit::record(
            &txn,
            Actor {
                user_id: user.id,
                impersonated_user_id: None,
            },
            None,
            "identity.auto_linked",
            "user",
            Some(user.id),
            json!({ "provider": provider.as_str() }),
        )
        .await?;
        let code = issue_exchange_code(&txn, user.id, client_label).await?;
        txn.commit().await?;
        return Ok(CallbackOutcome::LoggedIn {
            exchange_code: code,
        });
    }
    txn.commit().await?;

    // 4. Untrusted email: the account holder confirms from their mailbox.
    send_link_confirmation(state, &user, &identity).await?;
    Ok(CallbackOutcome::ConfirmationSent)
}

async fn find_identity<C: ConnectionTrait>(
    conn: &C,
    provider: Provider,
    subject: &str,
) -> Result<Option<login_identities::Model>, AppError> {
    Ok(LoginIdentities::find()
        .filter(login_identities::Column::Provider.eq(provider.as_str()))
        .filter(login_identities::Column::Subject.eq(subject))
        .one(conn)
        .await?)
}

async fn touch_identity<C: ConnectionTrait>(
    conn: &C,
    identity: login_identities::Model,
) -> Result<(), AppError> {
    let mut active: login_identities::ActiveModel = identity.into();
    active.last_used_at = Set(Some(now()));
    active.update(conn).await?;
    Ok(())
}

async fn insert_identity<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    identity: &VerifiedIdentity,
) -> Result<login_identities::Model, AppError> {
    let created = now();
    // UNIQUE (provider, subject) and (user_id, provider) back this up → 409.
    Ok(login_identities::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        provider: Set(identity.provider.as_str().to_string()),
        subject: Set(identity.subject.clone()),
        email_at_link: Set(identity.email.clone()),
        created_at: Set(created),
        last_used_at: Set(Some(created)),
    }
    .insert(conn)
    .await?)
}

/// Link flow (R10): the user is authenticated (the flow is bound to them)
/// AND proved the provider account — no email match needed.
async fn link_to_user(
    state: &AppState,
    user_id: Uuid,
    identity: &VerifiedIdentity,
) -> Result<CallbackOutcome, AppError> {
    let txn = state.db.begin().await?;
    if let Some(existing) = find_identity(&txn, identity.provider, &identity.subject).await? {
        return Ok(if existing.user_id == user_id {
            CallbackOutcome::Linked
        } else {
            CallbackOutcome::AlreadyUsed
        });
    }
    let has_same_provider = LoginIdentities::find()
        .filter(login_identities::Column::UserId.eq(user_id))
        .filter(login_identities::Column::Provider.eq(identity.provider.as_str()))
        .count(&txn)
        .await?
        > 0;
    if has_same_provider {
        // One identity per provider per account: unlink the old one first.
        return Ok(CallbackOutcome::AlreadyUsed);
    }
    insert_identity(&txn, user_id, identity).await?;
    audit::record(
        &txn,
        Actor {
            user_id,
            impersonated_user_id: None,
        },
        None,
        "identity.linked",
        "user",
        Some(user_id),
        json!({ "provider": identity.provider.as_str() }),
    )
    .await?;
    txn.commit().await?;
    Ok(CallbackOutcome::Linked)
}

async fn issue_exchange_code<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    client_label: Option<String>,
) -> Result<String, AppError> {
    let code = tokens::generate();
    exchange_codes::ActiveModel {
        code_hash: Set(code.hash),
        user_id: Set(user_id),
        client_label: Set(client_label),
        expires_at: Set(now() + Duration::seconds(EXCHANGE_TTL_SECONDS)),
        consumed_at: Set(None),
    }
    .insert(conn)
    .await?;
    Ok(code.token)
}

/// Trades a one-time exchange code for a session (single use, 60 s).
pub async fn exchange(state: &AppState, code: &str) -> Result<(String, users::Model), AppError> {
    let invalid = || AppError::Unauthorized("invalid or expired login code".to_string());
    let hash = tokens::hash(code);
    let txn = state.db.begin().await?;
    let consumed = ExchangeCodes::update_many()
        .col_expr(exchange_codes::Column::ConsumedAt, Expr::value(Some(now())))
        .filter(exchange_codes::Column::CodeHash.eq(hash.clone()))
        .filter(exchange_codes::Column::ConsumedAt.is_null())
        .filter(exchange_codes::Column::ExpiresAt.gt(now()))
        .exec(&txn)
        .await?;
    if consumed.rows_affected != 1 {
        return Err(invalid());
    }
    let row = ExchangeCodes::find_by_id(hash)
        .one(&txn)
        .await?
        .ok_or_else(invalid)?;
    let user = user_service::lock_for_update(&txn, row.user_id).await?;
    if user.status != users::STATUS_ACTIVE {
        return Err(invalid());
    }
    let user = user_service::record_login(&txn, user, false).await?;
    let (token, _) =
        sessions::create_login_session(&txn, &state.config, user.id, row.client_label).await?;
    txn.commit().await?;
    Ok((token, user))
}

/// R8 case 4: emails the account holder a link that, once clicked, links
/// the identity and logs them in. Rate-limited per account address so an
/// attacker-controlled provider account cannot flood a victim's inbox.
async fn send_link_confirmation(
    state: &AppState,
    user: &users::Model,
    identity: &VerifiedIdentity,
) -> Result<(), AppError> {
    let Some(to) = user.email.clone() else {
        return Ok(());
    };
    if !state.rate.check(
        &format!("ml:email:{to}"),
        3,
        StdDuration::from_secs(15 * 60),
    ) {
        tracing::warn!(user_id = %user.id, "link confirmation rate limited");
        return Ok(());
    }
    let token = tokens::generate();
    let created = now();
    login_magic_links::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user.id),
        token_hash: Set(token.hash),
        expires_at: Set(created + Duration::hours(state.config.auth.magic_link_hours)),
        consumed_at: Set(None),
        invalidated_at: Set(None),
        created_at: Set(created),
        purpose: Set(login_magic_links::PURPOSE_CONFIRM_LINK.to_string()),
        payload: Set(json!({
            "provider": identity.provider.as_str(),
            "subject": identity.subject,
            "email": identity.email,
        })),
    }
    .insert(&state.db)
    .await?;
    let url = format!("{}/auth/ml/{}", state.config.base_url, token.token);
    let (subject, body) = emails::link_confirmation_email(
        &user.first_name,
        identity.provider.label(),
        &url,
        state.config.auth.magic_link_hours,
    );
    if let Err(error) = state.mailer.send(&to, &subject, body).await {
        tracing::error!(?error, "failed to send link confirmation email");
    }
    Ok(())
}

/// Applies a clicked `confirm_link` magic link (called by the magic-link
/// consumer inside its transaction).
pub async fn apply_confirmed_link<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let invalid = || AppError::Unauthorized("invalid or expired login link".to_string());
    let provider = payload["provider"]
        .as_str()
        .and_then(Provider::parse)
        .ok_or_else(invalid)?;
    let subject = payload["subject"].as_str().ok_or_else(invalid)?.to_string();
    let identity = VerifiedIdentity {
        provider,
        subject,
        email: payload["email"].as_str().map(str::to_string),
        email_trusted: false,
    };
    match find_identity(conn, provider, &identity.subject).await? {
        Some(existing) if existing.user_id == user_id => return Ok(()),
        Some(_) => {
            return Err(AppError::Conflict(
                "this login is already used by another account".to_string(),
            ))
        }
        None => {}
    }
    insert_identity(conn, user_id, &identity).await?;
    audit::record(
        conn,
        Actor {
            user_id,
            impersonated_user_id: None,
        },
        None,
        "identity.linked",
        "user",
        Some(user_id),
        json!({ "provider": provider.as_str(), "via": "email_confirmation" }),
    )
    .await?;
    Ok(())
}

/// A user's linked identities (profile "login methods").
pub async fn list_for_user<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<login_identities::Model>, AppError> {
    Ok(LoginIdentities::find()
        .filter(login_identities::Column::UserId.eq(user_id))
        .order_by_asc(login_identities::Column::CreatedAt)
        .all(conn)
        .await?)
}

/// Unlinks one of the user's identities, keeping at least one login method
/// (R11): an email (magic link) or another identity.
pub async fn unlink(state: &AppState, user_id: Uuid, identity_id: Uuid) -> Result<(), AppError> {
    let txn = state.db.begin().await?;
    let user = user_service::lock_for_update(&txn, user_id).await?;
    let identities = list_for_user(&txn, user_id).await?;
    let target = identities
        .iter()
        .find(|i| i.id == identity_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("login method not found".to_string()))?;
    if user.email.is_none() && identities.len() == 1 {
        return Err(AppError::Conflict(
            "cannot remove your last login method".to_string(),
        ));
    }
    LoginIdentities::delete_by_id(target.id).exec(&txn).await?;
    audit::record(
        &txn,
        Actor {
            user_id,
            impersonated_user_id: None,
        },
        None,
        "identity.unlinked",
        "user",
        Some(user_id),
        json!({ "provider": target.provider }),
    )
    .await?;
    txn.commit().await?;
    Ok(())
}
