//! Self-service identity editing (`PUT /me/identity`): bootstrap accounts
//! start with an email-derived name; the person fixes it themselves (it also
//! feeds the licence-import identity binding).

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::test_support::TestApp;

#[tokio::test]
async fn user_renames_themselves_and_me_reflects_it() {
    let app = TestApp::new().await;
    let user = app.create_user("marc@example.com", true, false).await;
    let token = app.session_for(user.id).await;

    let (status, body) = app
        .request(
            Method::PUT,
            "/api/v1/me/identity",
            Some(&token),
            Some(json!({ "first_name": "  Marc ", "last_name": "Dupont" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["first_name"], "Marc", "trimmed");
    assert_eq!(body["last_name"], "Dupont");

    let (_, me) = app
        .request(Method::GET, "/api/v1/me", Some(&token), None)
        .await;
    assert_eq!(me["user"]["first_name"], "Marc");
    assert_eq!(me["user"]["last_name"], "Dupont");

    // Empty names are refused (they feed the licence identity binding).
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/me/identity",
            Some(&token),
            Some(json!({ "first_name": "Marc", "last_name": "   " })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn renaming_is_refused_while_impersonating() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("idn").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let admin_token = app.session_for(admin.id).await;

    let (_, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/idn/impersonate",
            Some(&admin_token),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    let imp_token = body["token"].as_str().unwrap().to_string();
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/me/identity",
            Some(&imp_token),
            Some(json!({ "first_name": "X", "last_name": "Y" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
