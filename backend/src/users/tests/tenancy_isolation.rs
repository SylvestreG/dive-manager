//! The new IDOR surface of this module: `/clubs/{slug}/members/{user_id}/...`
//! addresses USER-GLOBAL data through a club route. Every path must 404 when
//! the target is not an ACTIVE member of the addressed club.

use axum::http::{Method, StatusCode};
use sea_orm::*;
use serde_json::json;

use crate::tenancy::entities::memberships;
use crate::test_support::TestApp;

#[tokio::test]
async fn club_admin_cannot_read_profiles_outside_their_club() {
    let app = TestApp::new().await;
    let (club_a, admin_a, _) = app.create_club("club-a").await;
    let (club_b, _, member_b) = app.create_club("club-b").await;

    let admin = app.create_user("admin-a@example.com", true, false).await;
    app.add_membership(club_a.id, admin.id, admin_a.id).await;
    let token = app.session_for(admin.id).await;

    // Target belongs to club B only.
    let target = app.create_user("target@example.com", true, false).await;
    app.add_membership(club_b.id, target.id, member_b.id).await;

    // Reading through club A → 404 (never reveal the user exists).
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/club-a/members/{}/profile", target.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Same for writes: profile (carries the CACI), certifications, training
    // status, stamp.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/club-a/members/{}/profile", target.id),
            Some(&token),
            Some(json!({
                "birthdate": "1990-01-01", "phone": null, "licence_number": null,
                "instructor_number": null, "medical_certificate_valid_until": "2030-01-01",
                "gear_regulator": true, "gear_stab": true, "gear_stab_size": null,
                "gear_nitrox": false, "gear_second_regulator": false,
                "preparing_certification": null
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/club-a/members/{}/certifications", target.id),
            Some(&token),
            Some(json!({ "certification": "N1" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/club-a/members/{}/training-status", target.id),
            Some(&token),
            Some(json!({ "stagiaire_mf1_valid_until": "2027-01-01" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/clubs/club-a/members/{}/stamp", target.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn suspended_member_profile_is_not_visible() {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("club-s").await;
    let admin = app.create_user("admin-s@example.com", true, false).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    let token = app.session_for(admin.id).await;

    let target = app.create_user("suspended@example.com", true, false).await;
    let membership_id = app.add_membership(club.id, target.id, member_role.id).await;
    let membership = memberships::Entity::find_by_id(membership_id)
        .one(&app.state.db)
        .await
        .unwrap()
        .unwrap();
    let mut active: memberships::ActiveModel = membership.into();
    active.status = Set(memberships::STATUS_SUSPENDED.to_string());
    active.update(&app.state.db).await.unwrap();

    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/club-s/members/{}/profile", target.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn plain_member_cannot_read_or_edit_other_profiles() {
    let app = TestApp::new().await;
    let (club, _, member_role) = app.create_club("club-m").await;
    let caller = app.create_user("member1@example.com", true, false).await;
    let other = app.create_user("member2@example.com", true, false).await;
    app.add_membership(club.id, caller.id, member_role.id).await;
    app.add_membership(club.id, other.id, member_role.id).await;
    let token = app.session_for(caller.id).await;

    let (status, _) = app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/club-m/members/{}/profile", other.id),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/club-m/members/{}/certifications", other.id),
            Some(&token),
            Some(json!({ "certification": "N1" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
