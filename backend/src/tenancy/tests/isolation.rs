use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::test_support::TestApp;

#[tokio::test]
async fn tenant_isolation_blocks_cross_club_access() {
    let app = TestApp::new().await;
    let (club_a, admin_role_a, _) = app.create_club("club-a").await;
    let (club_b, admin_role_b, member_role_b) = app.create_club("club-b").await;
    let admin_a = app.create_user("admin-a@example.com", true, false).await;
    app.add_membership(club_a.id, admin_a.id, admin_role_a.id)
        .await;
    let token_a = app.session_for(admin_a.id).await;

    // Admin of A has no access to B at all.
    for path in [
        "/api/v1/clubs/club-b",
        "/api/v1/clubs/club-b/roles",
        "/api/v1/clubs/club-b/members",
        "/api/v1/clubs/club-b/audit-log",
    ] {
        let (status, _) = app.request(Method::GET, path, Some(&token_a), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }

    // IDOR: club-B resource ids under club A's URL must be invisible (404).
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/club-a/roles/{}/permissions", admin_role_b.id),
            Some(&token_a),
            Some(json!({ "permissions": [] })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let member_b = app.create_user("member-b@example.com", true, false).await;
    let membership_b = app
        .add_membership(club_b.id, member_b.id, member_role_b.id)
        .await;
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/club-a/memberships/{membership_b}"),
            Some(&token_a),
            Some(json!({ "status": "suspended" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cross-club role id in an invitation is rejected.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/club-a/invitations",
            Some(&token_a),
            Some(json!({
                "email": "new@example.com",
                "first_name": "New",
                "last_name": "Member",
                "role_id": member_role_b.id
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A global admin can act in any club (audited via_global_admin path).
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/club-b/roles",
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn suspended_club_and_membership_are_denied() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("usi").await;
    let member = app.create_user("member@example.com", true, false).await;
    let member_membership = app.add_membership(club.id, member.id, member_role.id).await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let member_token = app.session_for(member.id).await;
    let admin_token = app.session_for(admin.id).await;

    // Suspend the member's membership → denied.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/memberships/{member_membership}"),
            Some(&admin_token),
            Some(json!({ "status": "suspended" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(Method::GET, "/api/v1/clubs/usi", Some(&member_token), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Suspend the whole club → nobody gets in through club routes.
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/platform/clubs/{}", club.id),
            Some(&root_token),
            Some(json!({ "status": "suspended" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(Method::GET, "/api/v1/clubs/usi", Some(&admin_token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
