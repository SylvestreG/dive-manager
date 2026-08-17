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
