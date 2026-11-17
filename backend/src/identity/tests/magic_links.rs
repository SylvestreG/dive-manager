use axum::http::{Method, StatusCode};
use sea_orm::*;
use serde_json::json;

use crate::identity::entities::{login_magic_links, prelude::*};
use crate::test_support::TestApp;

#[tokio::test]
async fn magic_link_flow_activates_and_is_single_use() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", false, false).await;

    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link",
            None,
            Some(json!({ "email": "Diver@Example.com" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let token = app.last_email_token("/auth/ml/");
    let (status, body) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": token })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let session_token = body["token"].as_str().unwrap().to_string();

    // First login recorded, mailbox proven, and /me works.
    let stored = Users::find_by_id(user.id)
        .one(&app.state.db)
        .await
        .unwrap()
        .unwrap();
    assert!(stored.activated_at.is_some());
    assert!(stored.email_verified_at.is_some());
    let (status, me) = app
        .request(Method::GET, "/api/v1/me", Some(&session_token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["id"].as_str().unwrap(), user.id.to_string());

    // Strictly single use.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": token })),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn magic_link_newer_request_invalidates_pending_link() {
    let app = TestApp::new().await;
    app.create_user("diver@example.com", true, false).await;

    // Older link dies when a newer one is requested.
    let post = |email: &'static str| {
        app.request(
            Method::POST,
            "/api/v1/auth/magic-link",
            None,
            Some(json!({ "email": email })),
        )
    };
    post("diver@example.com").await;
    let first = app.last_email_token("/auth/ml/");
    post("diver@example.com").await;
    let second = app.last_email_token("/auth/ml/");
    assert_ne!(first, second);
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": first })),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The surviving link logs in once.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": second })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn magic_link_requests_are_rate_limited_silently() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", true, false).await;
    for _ in 0..5 {
        let (status, _) = app
            .request(
                Method::POST,
                "/api/v1/auth/magic-link",
                None,
                Some(json!({ "email": "diver@example.com" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "generic response, no enumeration");
    }
    let links = LoginMagicLinks::find()
        .filter(login_magic_links::Column::UserId.eq(user.id))
        .count(&app.state.db)
        .await
        .unwrap();
    assert_eq!(links, 3, "only 3 links per 15 minutes");
}
