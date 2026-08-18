use axum::http::{Method, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::test_support::TestApp;

#[tokio::test]
async fn member_without_permission_is_denied_everywhere() {
    let app = TestApp::new().await;
    let (club, _, member_role) = app.create_club("usi").await;
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
        (Method::GET, "/api/v1/clubs/usi/members", None),
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
    let (club, admin_role, _) = app.create_club("usi").await;
    let staff_role = app
        .create_role(club.id, "Staff", &["club.members.view"])
        .await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let staff = app.create_user("staff@example.com", true, false).await;
    app.add_membership(club.id, staff.id, staff_role.id).await;
    let admin_token = app.session_for(admin.id).await;
    let staff_token = app.session_for(staff.id).await;

    // Unknown permission strings are rejected, not silently dropped.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", staff_role.id),
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

    // Revoking the staff role's members.view takes effect on the next request.
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/usi/members",
            Some(&staff_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "staff role has members.view");
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", staff_role.id),
            Some(&admin_token),
            Some(json!({ "permissions": [] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/usi/members",
            Some(&staff_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "revocation is immediate");
}

#[tokio::test]
async fn existing_users_can_be_added_directly_including_the_global_admin() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let token = app.session_for(root.id).await;

    // The product-owner scenario: a global admin creates a club and adds
    // HIMSELF as its administrator, without any email round-trip.
    let (_, club) = app
        .request(
            Method::POST,
            "/api/v1/platform/clubs",
            Some(&token),
            Some(json!({ "name": "USI", "slug": "usi" })),
        )
        .await;
    assert_eq!(club["slug"], "usi");
    let (_, roles) = app
        .request(Method::GET, "/api/v1/clubs/usi/roles", Some(&token), None)
        .await;
    let admin_role_id = roles
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["is_admin"] == true)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, membership) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&token),
            Some(json!({ "email": "root@example.com", "role_id": admin_role_id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{membership}");
    assert_eq!(membership["is_admin"], true);
    let (_, me) = app
        .request(Method::GET, "/api/v1/me", Some(&token), None)
        .await;
    assert_eq!(me["memberships"][0]["club_slug"], "usi");

    // Adding twice conflicts; unknown emails must use an invitation.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&token),
            Some(json!({ "email": "root@example.com", "role_id": admin_role_id })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&token),
            Some(json!({ "email": "nobody@example.com", "role_id": admin_role_id })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Granting admin while impersonating is refused (require_real).
    let member = app.create_user("member@example.com", true, false).await;
    let member_role_id = roles
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Membre")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, added) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&token),
            Some(json!({ "email": "member@example.com", "role_id": member_role_id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let (_, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/impersonate",
            Some(&token),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    let impersonation = body["token"].as_str().unwrap();
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(impersonation),
            Some(json!({ "email": "nobody2@example.com", "role_id": admin_role_id })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_intake_permission_allows_direct_add_but_not_admin_grant() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("usi").await;
    let staff_role = app.create_role(club.id, "Staff", &[]).await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let staff = app.create_user("staff@example.com", true, false).await;
    app.add_membership(club.id, staff.id, staff_role.id).await;
    let admin_token = app.session_for(admin.id).await;
    let staff_token = app.session_for(staff.id).await;
    app.create_user("candidate@example.com", true, false).await;

    // Without club.invitations.manage, direct add is denied.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&staff_token),
            Some(json!({ "email": "candidate@example.com", "role_id": member_role.id })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Grant the intake permission to the staff role → direct add of a plain
    // member works…
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/usi/roles/{}/permissions", staff_role.id),
            Some(&admin_token),
            Some(json!({ "permissions": ["club.invitations.manage"] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&staff_token),
            Some(json!({ "email": "candidate@example.com", "role_id": member_role.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // …but granting the ADMIN role still needs club.roles.manage.
    app.create_user("candidate2@example.com", true, false).await;
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&staff_token),
            Some(json!({ "email": "candidate2@example.com", "role_id": admin_role.id })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn module_toggles_are_seeded_validated_and_persisted() {
    let app = TestApp::new().await;
    let (club, admin_role, _) = app.create_club("usi").await;
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
    let (club, admin_role, member_role) = app.create_club("usi").await;
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
