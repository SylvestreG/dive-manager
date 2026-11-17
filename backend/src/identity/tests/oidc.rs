//! OIDC login & linking (ADR-0009, design R7–R12) against a local fake
//! provider: a real HTTP server issuing ES256-signed id_tokens and a JWKS —
//! the production verification code path runs unchanged.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use sea_orm::*;
use serde_json::{json, Value};

use crate::config::{AppleClientConfig, Config, OidcClientConfig, OidcEndpoints};
use crate::identity::entities::{login_identities, prelude::*, users};
use crate::identity::services::oidc::MICROSOFT_CONSUMER_TENANT;
use crate::test_support::TestApp;

/// Throwaway P-256 key of the FAKE provider (test-only, never a secret).
const FAKE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgiADaA1oe7pd2K1ju
O/OjbTsrSK69fP+NyJ1e08Gamu2hRANCAATGWZwYG6uoRa+zrdEzDhRj5CPnpGOU
nsvFR4cGhdbLKj6f4swX+By8uDIFK3xR+odBG9vWrp3nrUhnYDoxTE0T
-----END PRIVATE KEY-----";
const FAKE_JWK_X: &str = "xlmcGBurqEWvs63RMw4UY-Qj56RjlJ7LxUeHBoXWyyo";
const FAKE_JWK_Y: &str = "Pp_izBf4HLy4MgUrfFH6h0Eb29auneetSGdgOjFMTRM";
const ISSUER: &str = "https://fake-idp.test";
const GOOGLE_CLIENT: &str = "google-client";
const MICROSOFT_CLIENT: &str = "microsoft-client";
const APPLE_CLIENT: &str = "fr.divemanager.web";

fn der(pem: &str) -> Vec<u8> {
    let body: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .unwrap()
}

#[derive(Default)]
struct FakeState {
    /// Claims of the next id_token (the test sets them after `start`).
    next_claims: Option<Value>,
    /// Last form received by /token (PKCE/secret assertions).
    last_token_form: Option<HashMap<String, String>>,
}

type Shared = Arc<Mutex<FakeState>>;

async fn token(
    State(fake): State<Shared>,
    Form(form): Form<HashMap<String, String>>,
) -> Json<Value> {
    let mut fake = fake.lock().unwrap();
    let claims = fake.next_claims.clone().expect("test must set claims");
    fake.last_token_form = Some(form);
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some("fake-key".to_string());
    let id_token = jsonwebtoken::encode(
        &header,
        &claims,
        &EncodingKey::from_ec_der(&der(FAKE_KEY_PEM)),
    )
    .unwrap();
    Json(json!({ "id_token": id_token, "access_token": "unused", "token_type": "Bearer" }))
}

async fn jwks() -> Json<Value> {
    Json(json!({ "keys": [{
        "kty": "EC", "crv": "P-256", "x": FAKE_JWK_X, "y": FAKE_JWK_Y,
        "kid": "fake-key", "alg": "ES256", "use": "sig"
    }]}))
}

struct Harness {
    app: TestApp,
    fake: Shared,
}

async fn harness() -> Harness {
    let fake: Shared = Arc::default();
    let router = Router::new()
        .route("/token", post(token))
        .route("/jwks", get(jwks))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

    let endpoints = |issuer: &str| OidcEndpoints {
        authorize_url: format!("{ISSUER}/authorize"),
        token_url: format!("http://{addr}/token"),
        jwks_url: format!("http://{addr}/jwks"),
        issuer: issuer.to_string(),
    };
    let key_dir = std::env::temp_dir().join(format!("dm-apple-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&key_dir).unwrap();
    let key_path = key_dir.join("apple.p8");
    std::fs::write(&key_path, FAKE_KEY_PEM).unwrap();

    let app = TestApp::with_config(|config: &mut Config| {
        config.oidc.google = Some(OidcClientConfig {
            client_id: GOOGLE_CLIENT.to_string(),
            client_secret: "google-secret".to_string(),
            endpoints: Some(endpoints(ISSUER)),
        });
        config.oidc.microsoft = Some(OidcClientConfig {
            client_id: MICROSOFT_CLIENT.to_string(),
            client_secret: "ms-secret".to_string(),
            endpoints: Some(endpoints(&format!("{ISSUER}/{{tid}}/v2.0"))),
        });
        config.oidc.apple = Some(AppleClientConfig {
            client_id: APPLE_CLIENT.to_string(),
            team_id: "TEAM123".to_string(),
            key_id: "KEY123".to_string(),
            private_key_path: key_path.to_string_lossy().to_string(),
            endpoints: Some(endpoints(ISSUER)),
        });
    })
    .await;
    Harness { app, fake }
}

fn query(url: &str) -> HashMap<String, String> {
    reqwest::Url::parse(url)
        .unwrap()
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

impl Harness {
    /// Runs start → (provider) → callback; `claims(nonce)` builds the
    /// id_token claims. Returns the frontend redirect target.
    async fn login(&self, provider: &str, claims: impl FnOnce(&str) -> Value) -> String {
        let (status, location) = self
            .app
            .navigate(
                Method::GET,
                &format!("/api/v1/auth/oidc/{provider}/start"),
                None,
            )
            .await;
        assert!(status.is_redirection(), "start: {status}");
        self.callback(provider, &location, claims).await
    }

    async fn callback(
        &self,
        provider: &str,
        authorize_url: &str,
        claims: impl FnOnce(&str) -> Value,
    ) -> String {
        let params = query(authorize_url);
        self.fake.lock().unwrap().next_claims = Some(claims(&params["nonce"]));
        let state = &params["state"];
        let (_, location) = if provider == "apple" {
            self.app
                .navigate(
                    Method::POST,
                    "/api/v1/auth/oidc/apple/callback",
                    Some(&format!("code=fake-code&state={state}")),
                )
                .await
        } else {
            self.app
                .navigate(
                    Method::GET,
                    &format!("/api/v1/auth/oidc/{provider}/callback?code=fake-code&state={state}"),
                    None,
                )
                .await
        };
        location
    }

    async fn exchange(&self, location: &str) -> (StatusCode, Value) {
        let code = location
            .split("code=")
            .nth(1)
            .expect("exchange code in redirect");
        self.app
            .request(
                Method::POST,
                "/api/v1/auth/exchange",
                None,
                Some(json!({ "code": code })),
            )
            .await
    }
}

fn claims(aud: &str, nonce: &str, sub: &str, email: &str, verified: Value) -> Value {
    let now = chrono::Utc::now().timestamp();
    json!({
        "iss": ISSUER, "aud": aud, "sub": sub, "nonce": nonce,
        "email": email, "email_verified": verified,
        "iat": now, "exp": now + 300
    })
}

async fn identities_of(app: &TestApp, user_id: uuid::Uuid) -> Vec<login_identities::Model> {
    LoginIdentities::find()
        .filter(login_identities::Column::UserId.eq(user_id))
        .all(&app.state.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn trusted_google_email_auto_links_then_subject_wins() {
    let h = harness().await;
    let user = h.app.create_user("diver@example.com", false, false).await;

    let location = h
        .login("google", |n| {
            claims(GOOGLE_CLIENT, n, "g-1", "Diver@Example.com", json!(true))
        })
        .await;
    assert!(
        location.starts_with("http://test/auth/done?code="),
        "{location}"
    );

    // PKCE: the verifier was sent to the token endpoint.
    let form = h.fake.lock().unwrap().last_token_form.clone().unwrap();
    assert!(form["code_verifier"].len() >= 43);
    assert_eq!(form["client_secret"], "google-secret");

    let (status, session) = h.exchange(&location).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    assert_eq!(session["user"]["id"], user.id.to_string());
    let stored = Users::find_by_id(user.id)
        .one(&h.app.state.db)
        .await
        .unwrap()
        .unwrap();
    assert!(stored.activated_at.is_some(), "first login recorded");
    assert_eq!(identities_of(&h.app, user.id).await.len(), 1);

    // Replaying the exchange code fails (single use).
    let (status, _) = h.exchange(&location).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Next login: matched by subject even if the provider email changed.
    let location = h
        .login("google", |n| {
            claims(GOOGLE_CLIENT, n, "g-1", "renamed@gmail.com", json!(true))
        })
        .await;
    let (status, session) = h.exchange(&location).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session["user"]["id"], user.id.to_string());
}

#[tokio::test]
async fn untrusted_email_needs_mailbox_confirmation() {
    let h = harness().await;
    let user = h.app.create_user("diver@example.com", true, false).await;

    // Google with email_verified=false is untrusted.
    let location = h
        .login("google", |n| {
            claims(GOOGLE_CLIENT, n, "g-2", "diver@example.com", json!(false))
        })
        .await;
    assert_eq!(location, "http://test/auth/check-email");
    assert!(
        identities_of(&h.app, user.id).await.is_empty(),
        "not linked yet"
    );

    let mail = h.app.state.mailer.captured().last().unwrap().clone();
    assert_eq!(mail.to, "diver@example.com");
    assert!(mail.subject.contains("Google"));
    let token = h.app.last_email_token("/auth/ml/");
    let (status, session) = h
        .app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": token })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let linked = identities_of(&h.app, user.id).await;
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].subject, "g-2");

    // Now the subject logs in directly.
    let location = h
        .login("google", |n| {
            claims(GOOGLE_CLIENT, n, "g-2", "diver@example.com", json!(false))
        })
        .await;
    assert!(
        location.starts_with("http://test/auth/done?code="),
        "{location}"
    );
}

#[tokio::test]
async fn microsoft_trust_depends_on_tenant_and_issuer_is_per_tenant() {
    let h = harness().await;
    let user = h.app.create_user("someone@outlook.com", false, false).await;
    let ms = |nonce: &str, tid: &str, iss_tid: &str| {
        let mut c = claims(
            MICROSOFT_CLIENT,
            nonce,
            &format!("ms-{tid}"),
            "someone@outlook.com",
            Value::Null,
        );
        c["tid"] = json!(tid);
        c["iss"] = json!(format!("{ISSUER}/{iss_tid}/v2.0"));
        c
    };

    // Issuer of another tenant than the token's tid → rejected.
    let location = h
        .login("microsoft", |n| {
            ms(n, MICROSOFT_CONSUMER_TENANT, "other-tenant")
        })
        .await;
    assert_eq!(location, "http://test/login?error=sso");

    // Work tenant: email not ownership-proven → confirmation.
    let location = h
        .login("microsoft", |n| ms(n, "work-tenant", "work-tenant"))
        .await;
    assert_eq!(location, "http://test/auth/check-email");

    // Consumer tenant (outlook.com): trusted → auto-link.
    let location = h
        .login("microsoft", |n| {
            ms(n, MICROSOFT_CONSUMER_TENANT, MICROSOFT_CONSUMER_TENANT)
        })
        .await;
    assert!(
        location.starts_with("http://test/auth/done?code="),
        "{location}"
    );
    assert_eq!(identities_of(&h.app, user.id).await.len(), 1);
}

#[tokio::test]
async fn apple_form_post_with_signed_client_secret() {
    let h = harness().await;
    let user = h.app.create_user("diver@icloud.com", false, false).await;
    let location = h
        .login("apple", |n| {
            claims(
                APPLE_CLIENT,
                n,
                "apple-1",
                "diver@icloud.com",
                json!("true"),
            )
        })
        .await;
    assert!(
        location.starts_with("http://test/auth/done?code="),
        "{location}"
    );
    assert_eq!(identities_of(&h.app, user.id).await.len(), 1);

    // The client secret is an ES256 JWT signed with the .p8 key.
    let secret = h.fake.lock().unwrap().last_token_form.clone().unwrap()["client_secret"].clone();
    let header = jsonwebtoken::decode_header(&secret).unwrap();
    assert_eq!(header.alg, Algorithm::ES256);
    assert_eq!(header.kid.as_deref(), Some("KEY123"));
    let jwk: jsonwebtoken::jwk::Jwk = serde_json::from_value(json!({
        "kty": "EC", "crv": "P-256", "x": FAKE_JWK_X, "y": FAKE_JWK_Y
    }))
    .unwrap();
    let mut validation = jsonwebtoken::Validation::new(Algorithm::ES256);
    validation.set_audience(&["https://appleid.apple.com"]);
    validation.set_issuer(&["TEAM123"]);
    let decoded = jsonwebtoken::decode::<Value>(
        &secret,
        &jsonwebtoken::DecodingKey::from_jwk(&jwk).unwrap(),
        &validation,
    )
    .expect("valid Apple client secret");
    assert_eq!(decoded.claims["sub"], APPLE_CLIENT);
}

#[tokio::test]
async fn hostile_tokens_and_replays_are_rejected() {
    let h = harness().await;
    h.app.create_user("diver@example.com", true, false).await;

    // Nonce mismatch.
    let location = h
        .login("google", |_| {
            claims(
                GOOGLE_CLIENT,
                "forged",
                "g-x",
                "diver@example.com",
                json!(true),
            )
        })
        .await;
    assert_eq!(location, "http://test/login?error=sso");

    // Token minted for another client (audience).
    let location = h
        .login("google", |n| {
            claims("someone-else", n, "g-x", "diver@example.com", json!(true))
        })
        .await;
    assert_eq!(location, "http://test/login?error=sso");

    // Expired token.
    let location = h
        .login("google", |n| {
            let mut c = claims(GOOGLE_CLIENT, n, "g-x", "diver@example.com", json!(true));
            c["exp"] = json!(chrono::Utc::now().timestamp() - 3600);
            c
        })
        .await;
    assert_eq!(location, "http://test/login?error=sso");

    // A state is single use, and bound to its provider.
    let (_, start) = h
        .app
        .navigate(Method::GET, "/api/v1/auth/oidc/google/start", None)
        .await;
    let ok = h
        .callback("google", &start, |n| {
            claims(GOOGLE_CLIENT, n, "g-y", "nobody@example.com", json!(true))
        })
        .await;
    assert_eq!(
        ok, "http://test/auth/unknown",
        "unknown identity, no account"
    );
    let replay = h
        .callback("google", &start, |n| {
            claims(GOOGLE_CLIENT, n, "g-y", "nobody@example.com", json!(true))
        })
        .await;
    assert_eq!(replay, "http://test/login?error=sso");

    // User cancelled at the provider.
    let (_, location) = h
        .app
        .navigate(
            Method::GET,
            "/api/v1/auth/oidc/google/callback?error=access_denied",
            None,
        )
        .await;
    assert_eq!(location, "http://test/login?error=sso_cancelled");
}

#[tokio::test]
async fn disabled_accounts_cannot_log_in_by_sso() {
    let h = harness().await;
    let user = h.app.create_user("diver@example.com", true, false).await;
    let mut active: users::ActiveModel = user.into();
    active.status = Set(users::STATUS_DISABLED.to_string());
    active.update(&h.app.state.db).await.unwrap();
    let location = h
        .login("google", |n| {
            claims(GOOGLE_CLIENT, n, "g-d", "diver@example.com", json!(true))
        })
        .await;
    assert_eq!(location, "http://test/login?error=sso");
}

#[tokio::test]
async fn profile_link_unlink_and_already_used() {
    let h = harness().await;
    let me = h.app.create_user("me@example.com", true, false).await;
    let token = h.app.session_for(me.id).await;
    let other = h.app.create_user("other@example.com", true, false).await;

    // Link Google from the profile (no email match needed).
    let (status, body) = h
        .app
        .request(
            Method::POST,
            "/api/v1/me/login-methods/google/link",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let location = h
        .callback("google", body["url"].as_str().unwrap(), |n| {
            claims(GOOGLE_CLIENT, n, "g-me", "personal@gmail.com", json!(true))
        })
        .await;
    assert_eq!(location, "http://test/profile?linked=google");

    let (_, methods) = h
        .app
        .request(Method::GET, "/api/v1/me/login-methods", Some(&token), None)
        .await;
    assert_eq!(methods["identities"][0]["provider"], "google");
    assert_eq!(methods["email"], "me@example.com");

    // The same Google account cannot be linked to another user.
    let other_token = h.app.session_for(other.id).await;
    let (_, body) = h
        .app
        .request(
            Method::POST,
            "/api/v1/me/login-methods/google/link",
            Some(&other_token),
            None,
        )
        .await;
    let location = h
        .callback("google", body["url"].as_str().unwrap(), |n| {
            claims(GOOGLE_CLIENT, n, "g-me", "personal@gmail.com", json!(true))
        })
        .await;
    assert_eq!(location, "http://test/profile?link_error=already_used");

    // Another user cannot remove my identity.
    let identity_id = methods["identities"][0]["id"].as_str().unwrap().to_string();
    let (status, _) = h
        .app
        .request(
            Method::DELETE,
            &format!("/api/v1/me/login-methods/{identity_id}"),
            Some(&other_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // I can (my email still allows magic links).
    let (status, _) = h
        .app
        .request(
            Method::DELETE,
            &format!("/api/v1/me/login-methods/{identity_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn last_login_method_cannot_be_removed() {
    let h = harness().await;
    // Member created without email, reaching the account only through SSO.
    let user = crate::identity::services::users::create_user(&h.app.state.db, None, "No", "Mail")
        .await
        .unwrap();
    let token = h.app.session_for(user.id).await;
    let (_, body) = h
        .app
        .request(
            Method::POST,
            "/api/v1/me/login-methods/google/link",
            Some(&token),
            None,
        )
        .await;
    h.callback("google", body["url"].as_str().unwrap(), |n| {
        claims(GOOGLE_CLIENT, n, "g-only", "x@gmail.com", json!(true))
    })
    .await;
    let identity = identities_of(&h.app, user.id).await.pop().unwrap();
    let (status, _) = h
        .app
        .request(
            Method::DELETE,
            &format!("/api/v1/me/login-methods/{}", identity.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn email_change_takes_effect_only_when_confirmed() {
    let h = harness().await;
    let me = h.app.create_user("old@example.com", true, false).await;
    let token = h.app.session_for(me.id).await;
    h.app.create_user("taken@example.com", true, false).await;

    let (status, _) = h
        .app
        .request(
            Method::PUT,
            "/api/v1/me/email",
            Some(&token),
            Some(json!({ "email": "taken@example.com" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _) = h
        .app
        .request(
            Method::PUT,
            "/api/v1/me/email",
            Some(&token),
            Some(json!({ "email": "New@Example.com" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.app.state.mailer.captured().last().unwrap().to,
        "new@example.com"
    );
    let unchanged = Users::find_by_id(me.id)
        .one(&h.app.state.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.email.as_deref(), Some("old@example.com"));

    let link = h.app.last_email_token("/auth/ml/");
    let (status, _) = h
        .app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": link })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let changed = Users::find_by_id(me.id)
        .one(&h.app.state.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(changed.email.as_deref(), Some("new@example.com"));
    assert!(changed.email_verified_at.is_some());
}

#[tokio::test]
async fn devices_are_listed_and_revocable_only_by_their_owner() {
    let h = harness().await;
    let me = h.app.create_user("me@example.com", true, false).await;
    let other = h.app.create_user("other@example.com", true, false).await;
    let token = h.app.session_for(me.id).await;
    let other_token = h.app.session_for(other.id).await;

    // A second device through SSO gets a label from its User-Agent.
    let location = h
        .login("google", |n| {
            claims(GOOGLE_CLIENT, n, "g-dev", "me@example.com", json!(true))
        })
        .await;
    h.exchange(&location).await;

    let (_, devices) = h
        .app
        .request(Method::GET, "/api/v1/me/sessions", Some(&token), None)
        .await;
    let devices = devices.as_array().unwrap().clone();
    assert_eq!(devices.len(), 2);
    assert!(devices.iter().any(|d| d["current"] == true));
    let other_device = devices.iter().find(|d| d["current"] == false).unwrap();
    assert_eq!(other_device["label"], "Chrome · Mac");
    let id = other_device["id"].as_str().unwrap();

    let (status, _) = h
        .app
        .request(
            Method::DELETE,
            &format!("/api/v1/me/sessions/{id}"),
            Some(&other_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "not your session");
    let (status, _) = h
        .app
        .request(
            Method::DELETE,
            &format!("/api/v1/me/sessions/{id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn providers_list_reflects_configuration() {
    let h = harness().await;
    let (status, body) = h
        .app
        .request(Method::GET, "/api/v1/auth/providers", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["google", "microsoft", "apple"]);

    let bare = TestApp::new().await;
    let (_, body) = bare
        .request(Method::GET, "/api/v1/auth/providers", None, None)
        .await;
    assert_eq!(body, json!([]));
    let (_, location) = bare
        .navigate(Method::GET, "/api/v1/auth/oidc/google/start", None)
        .await;
    assert_eq!(
        location, "http://test/login?error=sso",
        "unconfigured provider"
    );
}
