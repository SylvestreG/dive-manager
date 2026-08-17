use axum::http::{Method, StatusCode};
use sea_orm::*;
use serde_json::json;

use crate::tenancy::entities::{club_roles, prelude::*};
use crate::tenancy::services::seed;
use crate::test_support::TestApp;

#[tokio::test]
async fn impersonation_permissions_and_switch_semantics() {
    let app = TestApp::new().await;
    let (club, admin_role, _, member_role) = app.create_club("usi").await;
    let club_admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, club_admin.id, admin_role.id)
        .await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let root = app.create_user("root@example.com", true, true).await;

    // A member may not impersonate.
    let member_token = app.session_for(member.id).await;
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&member_token),
            Some(json!({ "user_id": club_admin.id })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Nobody impersonates a global admin.
    let admin_token = app.session_for(club_admin.id).await;
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&admin_token),
            Some(json!({ "user_id": root.id })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "root is not a club member");

    // Club admin impersonates a member of their club.
    let (status, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&admin_token),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let impersonation_token = body["token"].as_str().unwrap().to_string();

    let (_, me) = app
        .request(Method::GET, "/api/v1/me", Some(&impersonation_token), None)
        .await;
    assert_eq!(me["impersonating"], true);
    assert_eq!(me["user"]["email"], "member@example.com");
    assert_eq!(me["real_user"]["email"], "admin@example.com");

    // Sensitive actions refuse impersonating principals.
    let instructor_role = ClubRoles::find()
        .filter(club_roles::Column::ClubId.eq(club.id))
        .filter(club_roles::Column::Name.eq(seed::ROLE_INSTRUCTOR))
        .one(&app.state.db)
        .await
        .unwrap()
        .unwrap();
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", instructor_role.id),
            Some(&impersonation_token),
            Some(json!({ "permissions": [] })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Switch semantics: a global admin impersonating the club admin can jump
    // directly to the member; the old impersonation session dies.
    let root_token = app.session_for(root.id).await;
    let (_, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&root_token),
            Some(json!({ "user_id": club_admin.id })),
        )
        .await;
    let first_impersonation = body["token"].as_str().unwrap().to_string();
    let (status, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&first_impersonation),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "check runs against the REAL user");
    let second_impersonation = body["token"].as_str().unwrap().to_string();
    let (status, _) = app
        .request(Method::GET, "/api/v1/me", Some(&first_impersonation), None)
        .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "switched session is revoked"
    );

    // Stop: back to the real user on a fresh session.
    let (status, body) = app
        .request(
            Method::POST,
            "/api/v1/auth/stop-impersonation",
            Some(&second_impersonation),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["email"], "root@example.com");

    // A club admin of another club cannot impersonate into this club.
    let (club_b, admin_role_b, _, _) = app.create_club("club-b").await;
    let admin_b = app.create_user("admin-b@example.com", true, false).await;
    app.add_membership(club_b.id, admin_b.id, admin_role_b.id)
        .await;
    let token_b = app.session_for(admin_b.id).await;
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&token_b),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "no membership in club usi");

    // Platform endpoints refuse an impersonating global admin.
    let (_, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&root_token),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    let impersonating_root = body["token"].as_str().unwrap().to_string();
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/platform/clubs",
            Some(&impersonating_root),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn club_admin_impersonation_is_confined_to_the_club() {
    let app = TestApp::new().await;
    let (club_a, admin_role_a, _, member_role_a) = app.create_club("club-a").await;
    let (club_b, _, _, member_role_b) = app.create_club("club-b").await;
    let admin_a = app.create_user("admin-a@example.com", true, false).await;
    app.add_membership(club_a.id, admin_a.id, admin_role_a.id)
        .await;
    // The member belongs to BOTH clubs.
    let shared = app.create_user("shared@example.com", true, false).await;
    app.add_membership(club_a.id, shared.id, member_role_a.id)
        .await;
    app.add_membership(club_b.id, shared.id, member_role_b.id)
        .await;

    // Club admin A impersonates the shared member…
    let token_a = app.session_for(admin_a.id).await;
    let (status, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/club-a/impersonate",
            Some(&token_a),
            Some(json!({ "user_id": shared.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let impersonation = body["token"].as_str().unwrap().to_string();

    // …and may act as them in club A…
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/club-a",
            Some(&impersonation),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // …but NOT in club B, even though the member belongs to it.
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/club-b",
            Some(&impersonation),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // /me only reveals the club-A membership under a scoped impersonation.
    let (_, me) = app
        .request(Method::GET, "/api/v1/me", Some(&impersonation), None)
        .await;
    let memberships_shown = me["memberships"].as_array().unwrap();
    assert_eq!(memberships_shown.len(), 1);
    assert_eq!(memberships_shown[0]["club_slug"], "club-a");

    // A global admin's impersonation is unscoped and follows the member.
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/club-a/impersonate",
            Some(&root_token),
            Some(json!({ "user_id": shared.id })),
        )
        .await;
    let root_impersonation = body["token"].as_str().unwrap().to_string();
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/club-b",
            Some(&root_impersonation),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}
