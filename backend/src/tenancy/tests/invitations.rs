use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::test_support::TestApp;

#[tokio::test]
async fn club_creation_seeds_and_admin_invitation_grants_admin_membership() {
    let app = TestApp::new().await;
    let admin = app.create_user("root@example.com", true, true).await;
    let token = app.session_for(admin.id).await;

    let (status, club) = app
        .request(
            Method::POST,
            "/api/v1/platform/clubs",
            Some(&token),
            Some(json!({ "name": "USI Plongée", "slug": "USI" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(club["slug"], "usi", "slug is normalized lowercase");
    let club_id = club["id"].as_str().unwrap();

    // ADR-0007: exactly two seeded roles, and Membre starts with no
    // permission at all (fail closed).
    let (status, roles) = app
        .request(Method::GET, "/api/v1/clubs/usi/roles", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let roles = roles.as_array().unwrap();
    let names: Vec<&str> = roles.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Administrateur", "Membre"]);
    let membre = roles.iter().find(|r| r["name"] == "Membre").unwrap();
    assert_eq!(membre["is_admin"], false);
    assert_eq!(membre["permissions"].as_array().unwrap().len(), 0);

    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/platform/clubs/{club_id}/invitations"),
            Some(&token),
            Some(json!({
                "email": "president@example.com",
                "first_name": "Prés",
                "last_name": "Ident"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let invitation_token = app.last_email_token("/auth/invitations/");
    let (status, preview) = app
        .request(
            Method::GET,
            &format!("/api/v1/auth/invitations/{invitation_token}"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["club_name"], "USI Plongée");
    assert_eq!(preview["role_name"], "Administrateur");

    let (status, accepted) = app
        .request(
            Method::POST,
            &format!("/api/v1/auth/invitations/{invitation_token}/accept"),
            None,
            Some(json!({ "password": "chosen-password" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let president_token = accepted["token"].as_str().unwrap().to_string();

    let (status, me) = app
        .request(Method::GET, "/api/v1/me", Some(&president_token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["memberships"][0]["club_slug"], "usi");
    assert_eq!(me["memberships"][0]["is_club_admin"], true);

    // Accepting twice fails: single use.
    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/auth/invitations/{invitation_token}/accept"),
            None,
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The password chosen at activation works.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "president@example.com", "password": "chosen-password" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn invitation_resend_rotates_the_token_and_revoke_kills_it() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("usi").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let token = app.session_for(admin.id).await;

    let (status, invitation) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/invitations",
            Some(&token),
            Some(json!({
                "email": "newbie@example.com",
                "first_name": "New",
                "last_name": "Bie",
                "role_id": member_role.id
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let first_token = app.last_email_token("/auth/invitations/");
    let invitation_id = invitation["id"].as_str().unwrap().to_string();

    // Resend: a fresh token is issued, the old one dies.
    let (status, resent) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/usi/invitations/{invitation_id}/resend"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let second_token = app.last_email_token("/auth/invitations/");
    assert_ne!(first_token, second_token);
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/auth/invitations/{first_token}"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "old token is revoked");

    // Revoke the new invitation: its token stops working too.
    let new_id = resent["id"].as_str().unwrap();
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/clubs/usi/invitations/{new_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/auth/invitations/{second_token}/accept"),
            None,
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn direct_add_revokes_the_pending_invitation() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("usi").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let token = app.session_for(admin.id).await;
    app.create_user("dual@example.com", true, false).await;

    // Invite, then add the same person directly.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/invitations",
            Some(&token),
            Some(json!({
                "email": "dual@example.com",
                "first_name": "Du",
                "last_name": "Al",
                "role_id": member_role.id
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let invitation_token = app.last_email_token("/auth/invitations/");
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/memberships",
            Some(&token),
            Some(json!({ "email": "dual@example.com", "role_id": member_role.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // The pending invitation is gone: not listed, not acceptable.
    let (_, pending) = app
        .request(
            Method::GET,
            "/api/v1/clubs/usi/invitations",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(pending["total"], 0);
    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/auth/invitations/{invitation_token}/accept"),
            None,
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn inviting_an_existing_member_conflicts() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("usi").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let token = app.session_for(admin.id).await;

    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/clubs/usi/invitations",
            Some(&token),
            Some(json!({
                "email": "member@example.com",
                "first_name": "Al",
                "last_name": "Ready",
                "role_id": member_role.id
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}
