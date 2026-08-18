//! Authorization matrix of the users module: impersonation must never gain
//! writes on user-global data, view-only staff must not write, and global
//! admins keep their platform-wide reach.

use axum::http::{Method, StatusCode};
use serde_json::{json, Value};

use crate::test_support::TestApp;

#[tokio::test]
async fn me_writes_are_refused_while_impersonating() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("imp").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let admin_token = app.session_for(admin.id).await;

    let (status, body) = app
        .request(
            Method::POST,
            "/api/v1/clubs/imp/impersonate",
            Some(&admin_token),
            Some(json!({ "user_id": member.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let imp_token = body["token"].as_str().unwrap().to_string();

    // Reads stay allowed: impersonation is "view as".
    let (status, _) = app
        .request(Method::GET, "/api/v1/me/profile", Some(&imp_token), None)
        .await;
    assert_eq!(status, StatusCode::OK);

    // Every write on user-global data is refused (escalation guard: a role
    // with club.impersonate only must not gain profile writes).
    let writes: Vec<(Method, String, Option<Value>)> = vec![
        (
            Method::PUT,
            "/api/v1/me/profile".to_string(),
            Some(json!({
                "birthdate": "1990-01-01", "phone": null, "licence_number": null,
                "instructor_number": null, "medical_certificate_valid_until": null,
                "gear_regulator": true, "gear_stab": true, "gear_stab_size": null,
                "gear_nitrox": false, "gear_second_regulator": false,
                "preparing_certification": null
            })),
        ),
        (
            Method::PUT,
            "/api/v1/me/stamp".to_string(),
            Some(json!({ "image_base64": "aGVsbG8=" })),
        ),
        (Method::DELETE, "/api/v1/me/stamp".to_string(), None),
    ];
    for (method, path, body) in writes {
        let (status, _) = app
            .request(method.clone(), &path, Some(&imp_token), body)
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn view_only_staff_can_read_but_not_write_member_profiles() {
    let app = TestApp::new().await;
    let (club, _, member_role) = app.create_club("view").await;
    let staff_role = app
        .create_role(club.id, "Staff", &["club.members.view"])
        .await;
    let staff = app.create_user("staff@example.com", true, false).await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, staff.id, staff_role.id).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let token = app.session_for(staff.id).await;

    // The view-only staff persona holds club.members.view only (ADR-0007:
    // no seeded staff role — the test builds its own).
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/view/members/{}/profile", member.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let writes: Vec<(Method, String, Option<Value>)> = vec![
        (
            Method::PUT,
            format!("/api/v1/clubs/view/members/{}/profile", member.id),
            Some(json!({
                "birthdate": "1990-01-01", "phone": null, "licence_number": null,
                "instructor_number": null, "medical_certificate_valid_until": "2030-01-01",
                "gear_regulator": true, "gear_stab": true, "gear_stab_size": null,
                "gear_nitrox": false, "gear_second_regulator": false,
                "preparing_certification": null
            })),
        ),
        (
            Method::POST,
            format!("/api/v1/clubs/view/members/{}/certifications", member.id),
            Some(json!({ "certification": "N1" })),
        ),
        (
            Method::PUT,
            format!("/api/v1/clubs/view/members/{}/training-status", member.id),
            Some(json!({ "stagiaire_mf1_valid_until": "2029-01-01" })),
        ),
        (
            Method::DELETE,
            format!("/api/v1/clubs/view/members/{}/stamp", member.id),
            None,
        ),
    ];
    for (method, path, body) in writes {
        let (status, _) = app.request(method.clone(), &path, Some(&token), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn certified_instructors_see_their_club_members_read_only() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("e3").await;
    let admin = app.create_user("admin@example.com", true, false).await;
    let marc = app.create_user("marc@example.com", true, false).await;
    let eleve = app.create_user("eleve@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    app.add_membership(club.id, marc.id, member_role.id).await;
    app.add_membership(club.id, eleve.id, member_role.id).await;
    let admin_token = app.session_for(admin.id).await;
    let marc_token = app.session_for(marc.id).await;

    // Plain member (no grade): no list access.
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/e3/members",
            Some(&marc_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Staff records Marc's MF1 → certified E3 → derived READ access
    // (ADR-0007 amendment: "un encadrant voit les plongeurs de son club").
    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/e3/members/{}/certifications", marc.id),
            Some(&admin_token),
            Some(json!({ "certification": "MF1", "validated": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, list) = app
        .request(
            Method::GET,
            "/api/v1/clubs/e3/members",
            Some(&marc_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["total"], 3);
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/e3/members/{}/profile", eleve.id),
            Some(&marc_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // READ ONLY: writes still require club.members.manage.
    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/e3/members/{}/certifications", eleve.id),
            Some(&marc_token),
            Some(json!({ "certification": "N1" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A stagiaire MF1 (dated status, no certification) gets NOTHING
    // (decision Q5: no application rights).
    let stagiaire = app.create_user("stagiaire@example.com", true, false).await;
    app.add_membership(club.id, stagiaire.id, member_role.id)
        .await;
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/e3/members/{}/training-status", stagiaire.id),
            Some(&admin_token),
            Some(json!({ "stagiaire_mf1_valid_until": "2029-01-01" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let stagiaire_token = app.session_for(stagiaire.id).await;
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/clubs/e3/members",
            Some(&stagiaire_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn global_admin_reaches_member_profiles_without_membership() {
    let app = TestApp::new().await;
    let (club, _, member_role) = app.create_club("ga").await;
    let member = app.create_user("member@example.com", true, false).await;
    app.add_membership(club.id, member.id, member_role.id).await;
    let root = app.create_user("root@example.com", true, true).await;
    let token = app.session_for(root.id).await;

    // Not a member of the club: via_global_admin grants full permissions.
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/ga/members/{}/profile", member.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/ga/members/{}/certifications", member.id),
            Some(&token),
            Some(json!({ "certification": "N1", "validated": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
