use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::identity::services::login;
use crate::test_support::TestApp;

#[tokio::test]
async fn password_login_rejects_bad_and_unknown_credentials() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", true, false).await;
    login::set_password(&app.state.db, user.id, "s3cret-password")
        .await
        .unwrap();

    let login_req = |email: &'static str, password: &'static str| {
        app.request(
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": email, "password": password })),
        )
    };
    let (status, _) = login_req("diver@example.com", "wrong-password").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, unknown_body) = login_req("nobody@example.com", "whatever-pass").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status2, bad_body) = login_req("diver@example.com", "wrong-password").await;
    assert_eq!(status2, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_body, bad_body, "identical shape: no enumeration");
    let (status, _) = login_req("diver@example.com", "s3cret-password").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn login_methods_reveal_password_only_for_password_holders() {
    let app = TestApp::new().await;
    let no_password = app.create_user("magic@example.com", true, false).await;
    let with_password = app.create_user("pass@example.com", true, false).await;
    login::set_password(&app.state.db, with_password.id, "s3cret-password")
        .await
        .unwrap();
    let _ = no_password;

    let methods = |email: &'static str| {
        app.request(
            Method::POST,
            "/api/v1/auth/login-methods",
            None,
            Some(json!({ "email": email })),
        )
    };
    let (status, body) = methods("pass@example.com").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["password"], true);
    // Password-less and unknown accounts are indistinguishable.
    let (_, body) = methods("magic@example.com").await;
    assert_eq!(body["password"], false);
    let (_, body) = methods("unknown@example.com").await;
    assert_eq!(body["password"], false);

    // Rate limited per email: 20 lookups per window, then 429
    // (1 call above + 19 here = 20; the 21st must be rejected).
    for _ in 0..19 {
        methods("pass@example.com").await;
    }
    let (status, _) = methods("pass@example.com").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn short_password_is_rejected() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", true, false).await;
    let token = app.session_for(user.id).await;
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/me/password",
            Some(&token),
            Some(json!({ "password": "short" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn me_reports_whether_a_password_is_set() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", true, false).await;
    let token = app.session_for(user.id).await;

    let (_, me) = app
        .request(Method::GET, "/api/v1/me", Some(&token), None)
        .await;
    assert_eq!(me["has_password"], false);

    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/me/password",
            Some(&token),
            Some(json!({ "password": "s3cret-password" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, me) = app
        .request(Method::GET, "/api/v1/me", Some(&token), None)
        .await;
    assert_eq!(me["has_password"], true);
}
