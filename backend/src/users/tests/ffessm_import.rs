//! FFESSM licence import (`POST /me/ffessm/import`). Each member brings
//! their OWN official licence from their profile page (product decision
//! 2026-08-18): import writes profile fields and every mapped brevet with
//! `imported` provenance — but it must still respect the "provenance only
//! rises" rule (a club-`validated` row is never downgraded) and refuse
//! impersonating principals (user-global write).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, IntoActiveModel, Set};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::test_support::TestApp;
use crate::AppState;

/// The same anonymized page the parser unit-tests use.
const FIXTURE: &str = include_str!("../../ffessm_licence/testdata/licence_sample.html");

struct Ctx {
    app: TestApp,
    slug: String,
    member_id: Uuid,
    admin_token: String,
    member_token: String,
}

async fn setup() -> Ctx {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("ffessm").await;
    let admin = app.create_user("staff@example.com", true, false).await;
    let member = app.create_user("diver@example.com", true, false).await;
    // The identity-binding check compares the account name with the licence
    // holder ("Mr PRENOM NOM" in the fixture): give the member that name.
    let mut active = member.clone().into_active_model();
    active.first_name = Set("Prénom".to_string());
    active.last_name = Set("Nom".to_string());
    active.update(&app.state.db).await.expect("rename member");
    app.add_membership(club.id, admin.id, admin_role.id).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    Ctx {
        slug: club.slug.clone(),
        member_id: member.id,
        admin_token: app.session_for(admin.id).await,
        member_token: app.session_for(member.id).await,
        app,
    }
}

/// Launch a throwaway HTTP server that serves `FIXTURE` at the licence path,
/// and a router whose FFESSM base URL points at it.
async fn stub_router(app: &TestApp) -> Router {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stub = Router::new().route(
        "/Home/InfoLicence",
        get(|| async { axum::response::Html(FIXTURE) }),
    );
    tokio::spawn(async move {
        axum::serve(listener, stub).await.unwrap();
    });

    let mut cfg = (*app.state.config).clone();
    cfg.ffessm_infolicence_base_url = format!("http://{addr}");
    let state = AppState {
        db: app.state.db.clone(),
        config: Arc::new(cfg),
        mailer: app.state.mailer.clone(),
        rate: app.state.rate.clone(),
        oidc: app.state.oidc.clone(),
    };
    crate::app::build_router(state)
}

async fn post(router: &Router, path: &str, token: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

fn cert<'a>(profile: &'a Value, code: &str) -> Option<&'a Value> {
    profile["profile"]["certifications"]
        .as_array()?
        .iter()
        .find(|c| c["code"] == code)
}

#[tokio::test]
async fn member_imports_their_own_licence() {
    let ctx = setup().await;
    let router = stub_router(&ctx.app).await;

    let (status, body) = post(
        &router,
        "/api/v1/me/ffessm/import",
        &ctx.member_token,
        json!({ "number": "A-00-000000", "key": "TESTKEY" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    // Identity carried back so the member confirms it is their licence.
    assert_eq!(body["holder_name"], "Mr PRENOM NOM");
    assert_eq!(body["licence_number"], "A-00-000000");
    assert!(body["unmapped"].as_array().unwrap().is_empty());

    // All nine brevets imported.
    assert_eq!(body["imported"].as_array().unwrap().len(), 9);

    // Profile fields filled from the licence.
    assert_eq!(body["profile"]["profile"]["birthdate"], "1984-10-13");
    assert_eq!(body["profile"]["profile"]["licence_number"], "A-00-000000");

    // Every written cert carries `imported` provenance and its delivery date.
    let n5 = cert(&body, "N5").expect("N5 present");
    assert_eq!(n5["provenance"], "imported");
    assert_eq!(n5["obtained_on"], "2026-04-05");
    assert_eq!(cert(&body, "N1").unwrap()["provenance"], "imported");
}

#[tokio::test]
async fn import_keeps_a_club_validated_row_and_reports_it_skipped() {
    let ctx = setup().await;
    let router = stub_router(&ctx.app).await;

    // Staff validated N1 first (strongest provenance), through the club route.
    let (status, _) = post(
        &router,
        &format!(
            "/api/v1/clubs/{}/members/{}/certifications",
            ctx.slug, ctx.member_id
        ),
        &ctx.admin_token,
        json!({ "certification": "N1", "validated": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The member imports their licence themselves.
    let (status, body) = post(
        &router,
        "/api/v1/me/ffessm/import",
        &ctx.member_token,
        json!({ "number": "A-00-000000", "key": "TESTKEY" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    // N1 stays validated (not downgraded) and is reported as skipped.
    assert_eq!(cert(&body, "N1").unwrap()["provenance"], "validated");
    let skipped = body["skipped_stronger"].as_array().unwrap();
    assert!(skipped.iter().any(|c| c == "N1"), "N1 should be skipped");
    assert!(!body["imported"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c == "N1"));
}

#[tokio::test]
async fn someone_elses_licence_is_refused_and_nothing_is_written() {
    // Review S1: the ADMIN (named Test User) tries to import the fixture
    // licence of "Mr PRENOM NOM" onto their own profile → 422, no write.
    let ctx = setup().await;
    let router = stub_router(&ctx.app).await;

    let (status, body) = post(
        &router,
        "/api/v1/me/ffessm/import",
        &ctx.admin_token,
        json!({ "number": "A-00-000000", "key": "TESTKEY" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");

    // Nothing was written on the admin's profile.
    let (_, profile) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/me/profile",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    assert!(profile["certifications"].as_array().unwrap().is_empty());
    assert_eq!(profile["profile"]["licence_number"], Value::Null);
}

#[tokio::test]
async fn licence_with_a_different_birthdate_is_refused() {
    // Right name, wrong birthdate already on file → refused before writes.
    let ctx = setup().await;
    let router = stub_router(&ctx.app).await;
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.member_token),
            Some(json!({
                "birthdate": "1990-01-01", "phone": null, "licence_number": null,
                "instructor_number": null, "medical_certificate_valid_until": null,
                "gear_regulator": true, "gear_stab": true, "gear_stab_size": null,
                "gear_nitrox": false, "gear_second_regulator": false,
                "preparing_certification": null
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = post(
        &router,
        "/api/v1/me/ffessm/import",
        &ctx.member_token,
        json!({ "number": "A-00-000000", "key": "TESTKEY" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
}

#[tokio::test]
async fn upstream_redirect_means_wrong_key_not_an_error() {
    // A wrong/expired key answers 302 on the real service: mapped to 404.
    let ctx = setup().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stub = Router::new().route(
        "/Home/InfoLicence",
        get(|| async {
            (
                StatusCode::FOUND,
                [(axum::http::header::LOCATION, "/Home/Error")],
                "",
            )
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, stub).await.unwrap();
    });
    let mut cfg = (*ctx.app.state.config).clone();
    cfg.ffessm_infolicence_base_url = format!("http://{addr}");
    let state = AppState {
        db: ctx.app.state.db.clone(),
        config: Arc::new(cfg),
        mailer: ctx.app.state.mailer.clone(),
        rate: ctx.app.state.rate.clone(),
        oidc: ctx.app.state.oidc.clone(),
    };
    let router = crate::app::build_router(state);

    let (status, _) = post(
        &router,
        "/api/v1/me/ffessm/import",
        &ctx.member_token,
        json!({ "number": "A-00-000000", "key": "WRONGKEY" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn import_is_rate_limited_per_user() {
    // The limiter sits before target resolution: 30 (invalid) attempts pass
    // through it, the 31st answers 429 without any outbound call.
    let ctx = setup().await;
    for _ in 0..30 {
        let (status, _) = ctx
            .app
            .request(
                Method::POST,
                "/api/v1/me/ffessm/import",
                Some(&ctx.member_token),
                Some(json!({ "number": "not-a-number", "key": "!" })),
            )
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            "/api/v1/me/ffessm/import",
            Some(&ctx.member_token),
            Some(json!({ "number": "not-a-number", "key": "!" })),
        )
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn import_is_refused_while_impersonating_and_the_staff_route_is_gone() {
    // Guards run before any outbound fetch, so no stub needed.
    let ctx = setup().await;

    // Impersonating principals cannot write user-global data.
    let (status, body) = ctx
        .app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/{}/impersonate", ctx.slug),
            Some(&ctx.admin_token),
            Some(json!({ "user_id": ctx.member_id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let imp_token = body["token"].as_str().unwrap().to_string();
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            "/api/v1/me/ffessm/import",
            Some(&imp_token),
            Some(json!({ "number": "A-00-000000", "key": "TESTKEY" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The staff-side import endpoint no longer exists (self-service only).
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            &format!(
                "/api/v1/clubs/{}/members/{}/ffessm/import",
                ctx.slug, ctx.member_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "number": "A-00-000000", "key": "TESTKEY" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
