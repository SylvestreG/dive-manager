use axum::http::{Method, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::test_support::TestApp;

#[tokio::test]
async fn member_without_permission_is_denied_everywhere() {
    let app = TestApp::new().await;
    let (club, _, _, member_role) = app.create_club("usi").await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let token = app.session_for(member.id).await;

    // Members can see the club context itself...
    let (status, ctx) = app
        .request(Method::GET, "/api/v1/clubs/usi", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ctx["permissions"].as_array().unwrap().len(), 0);

    // ...and nothing that needs a permission (fail closed).
    let denied: Vec<(Method, &str, Option<Value>)> = vec![
        (Method::GET, "/api/v1/clubs/usi/roles", None),
        (Method::GET, "/api/v1/clubs/usi/memberships", None),
        (Method::GET, "/api/v1/clubs/usi/modules", None),
        (Method::GET, "/api/v1/clubs/usi/invitations", None),
        (Method::GET, "/api/v1/clubs/usi/audit-log", None),
        (
            Method::PUT,
            "/api/v1/clubs/usi",
            Some(json!({ "name": "Hacked" })),
        ),
        (
            Method::PUT,
            "/api/v1/clubs/usi/modules",
            Some(json!({ "module": "members", "enabled": true })),
        ),
        (
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(json!({ "user_id": Uuid::now_v7() })),
        ),
    ];
    for (method, path, body) in denied {
        let (status, _) = app.request(method.clone(), path, Some(&token), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn role_permission_editing_is_validated_and_effective() {
    let app = TestApp::new().await;
    let (club, admin_role, instructor_role, _) = app.create_club("usi").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let instructor = app.create_user("encadrant@example.com", true, false).await;
    app.add_membership(club.id, instructor.id, instructor_role.id)
        .await;
    let admin_token = app.session_for(admin.id).await;
    let instructor_token = app.session_for(instructor.id).await;

    // Unknown permission strings are rejected, not silently dropped.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", instructor_role.id),
            Some(&admin_token),
            Some(json!({ "permissions": ["club.everything"] })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // The admin role's permissions are implicit and not editable.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", admin_role.id),
            Some(&admin_token),
            Some(json!({ "permissions": [] })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Revoking Encadrant's members.view takes effect on the next request.
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/usi/memberships",
            Some(&instructor_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "seeded Encadrant has members.view");
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", instructor_role.id),
            Some(&admin_token),
            Some(json!({ "permissions": [] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/usi/memberships",
            Some(&instructor_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "revocation is immediate");
}

#[tokio::test]
async fn module_toggles_are_seeded_validated_and_persisted() {
    let app = TestApp::new().await;
    let (club, admin_role, _, _) = app.create_club("usi").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let token = app.session_for(admin.id).await;

    // Seeded: every known module, disabled.
    let (status, modules) = app
        .request(Method::GET, "/api/v1/clubs/usi/modules", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let list = modules.as_array().unwrap();
    assert_eq!(list.len(), crate::tenancy::services::seed::MODULES.len());
    assert!(list.iter().all(|m| m["enabled"] == false));

    // Unknown module rejected; known module toggles and persists.
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/clubs/usi/modules",
            Some(&token),
            Some(json!({ "module": "blockchain", "enabled": true })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/clubs/usi/modules",
            Some(&token),
            Some(json!({ "module": "members", "enabled": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, modules) = app
        .request(Method::GET, "/api/v1/clubs/usi/modules", Some(&token), None)
        .await;
    let members = modules
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["module"] == "members")
        .unwrap();
    assert_eq!(members["enabled"], true);

    // The toggle left an audit trail (paginated envelope).
    let (status, audit) = app
        .request(
            Method::GET,
            "/api/v1/clubs/usi/audit-log",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(audit["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "club.module_toggled"));
    assert!(audit["total"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn last_active_admin_cannot_be_removed() {
    let app = TestApp::new().await;
    let (club, admin_role, _, member_role) = app.create_club("usi").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    let membership_id = app.add_membership(club.id, admin.id, admin_role.id).await;
    let token = app.session_for(admin.id).await;

    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/memberships/{membership_id}"),
            Some(&token),
            Some(json!({ "role_id": member_role.id })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // With a second admin, the demotion goes through.
    let second = app.create_user("admin2@example.com", true, false).await;
    app.add_membership(club.id, second.id, admin_role.id).await;
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/memberships/{membership_id}"),
            Some(&token),
            Some(json!({ "role_id": member_role.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}
