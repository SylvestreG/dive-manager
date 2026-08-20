//! Tenant isolation / IDOR: validations are user-global, but club routes
//! must never expose or touch a diver who is not an ACTIVE member of the
//! addressed club (404, never 403 — do not reveal existence elsewhere).

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::competencies::tests::support;
use crate::test_support::TestApp;

#[tokio::test]
async fn club_b_cannot_read_or_validate_club_a_members() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, skills) = support::build_tree(&app, &root_token, "N2").await;

    let (club_a, _, member_role_a) = app.create_club("cluba").await;
    let (club_b, admin_role_b, member_role_b) = app.create_club("clubb").await;
    let diver = app.create_user("diver@example.com", true, false).await;
    app.add_membership(club_a.id, diver.id, member_role_a.id)
        .await;

    // Club B's admin AND an E4 instructor of club B: the target is not a
    // member of B → 404 on every read and write.
    let b_admin = app.create_user("b-admin@example.com", true, false).await;
    app.add_membership(club_b.id, b_admin.id, admin_role_b.id)
        .await;
    let b_instructor = app.create_user("b-e4@example.com", true, false).await;
    app.add_membership(club_b.id, b_instructor.id, member_role_b.id)
        .await;
    support::make_instructor(&app, b_instructor.id, "E4").await;

    for token in [
        app.session_for(b_admin.id).await,
        app.session_for(b_instructor.id).await,
    ] {
        let (status, _) = app
            .request(
                Method::GET,
                &format!(
                    "/api/v1/clubs/clubb/members/{}/competencies?certification=N2",
                    diver.id
                ),
                Some(&token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = app
            .request(
                Method::GET,
                &format!(
                    "/api/v1/clubs/clubb/members/{}/competencies/document?certification=N2",
                    diver.id
                ),
                Some(&token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = app
            .request(
                Method::PUT,
                &format!(
                    "/api/v1/clubs/clubb/members/{}/skill-validations/{}",
                    diver.id, skills[0]
                ),
                Some(&token),
                Some(json!({ "stage": "vu" })),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // And club B's instructor cannot enter club A's context at all.
    let b_token = app.session_for(b_instructor.id).await;
    let (status, _) = app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/cluba/members/{}/skill-validations/{}",
                diver.id, skills[0]
            ),
            Some(&b_token),
            Some(json!({ "stage": "vu" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn suspended_members_are_invisible() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, skills) = support::build_tree(&app, &root_token, "N2").await;

    let (club, _, member_role) = app.create_club("susp").await;
    let diver = app.create_user("diver@example.com", true, false).await;
    let membership_id = app.add_membership(club.id, diver.id, member_role.id).await;
    let coach = app.create_user("coach@example.com", true, false).await;
    app.add_membership(club.id, coach.id, member_role.id).await;
    support::make_instructor(&app, coach.id, "E3").await;
    let coach_token = app.session_for(coach.id).await;

    // Active: visible and validatable.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/susp/members/{}/skill-validations/{}",
                diver.id, skills[0]
            ),
            Some(&coach_token),
            Some(json!({ "stage": "vu" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Suspended: 404 everywhere.
    {
        use crate::tenancy::entities::memberships;
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        let row = memberships::Entity::find_by_id(membership_id)
            .one(&app.state.db)
            .await
            .expect("query")
            .expect("membership");
        let mut active: memberships::ActiveModel = row.into();
        active.status = Set("suspended".to_string());
        active.update(&app.state.db).await.expect("suspend");
    }
    let (status, _) = app
        .request(
            Method::GET,
            &format!(
                "/api/v1/clubs/susp/members/{}/competencies?certification=N2",
                diver.id
            ),
            Some(&coach_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/susp/members/{}/skill-validations/{}",
                diver.id, skills[1]
            ),
            Some(&coach_token),
            Some(json!({ "stage": "vu" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn validations_follow_the_user_across_clubs() {
    // The user-global rule: validated in club A, visible from club B where
    // the diver is ALSO an active member.
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, skills) = support::build_tree(&app, &root_token, "N2").await;

    let (club_a, _, member_role_a) = app.create_club("multa").await;
    let (club_b, admin_role_b, member_role_b) = app.create_club("multb").await;
    let diver = app.create_user("diver@example.com", true, false).await;
    app.add_membership(club_a.id, diver.id, member_role_a.id)
        .await;
    app.add_membership(club_b.id, diver.id, member_role_b.id)
        .await;
    let coach = app.create_user("coach@example.com", true, false).await;
    app.add_membership(club_a.id, coach.id, member_role_a.id)
        .await;
    support::make_instructor(&app, coach.id, "E3").await;
    let coach_token = app.session_for(coach.id).await;

    let (status, _) = app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/multa/members/{}/skill-validations/{}",
                diver.id, skills[0]
            ),
            Some(&coach_token),
            Some(json!({ "stage": "valide" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let b_admin = app.create_user("b-admin@example.com", true, false).await;
    app.add_membership(club_b.id, b_admin.id, admin_role_b.id)
        .await;
    let b_token = app.session_for(b_admin.id).await;
    let (status, view) = app
        .request(
            Method::GET,
            &format!(
                "/api/v1/clubs/multb/members/{}/competencies?certification=N2",
                diver.id
            ),
            Some(&b_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["progress"]["validated"], 1);
}
