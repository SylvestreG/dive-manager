//! Business rules of the validation workflow: fixed stages per level,
//! append-only events, dates, notes, progress and the /me views.

use axum::http::{Method, StatusCode};
use serde_json::json;
use uuid::Uuid;

use crate::competencies::tests::support;
use crate::test_support::TestApp;

async fn setup(slug: &str, certification: &str) -> (TestApp, String, String, Uuid, Vec<Uuid>) {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, skill_ids) = support::build_tree(&app, &root_token, certification).await;

    let (club, _, member_role) = app.create_club(slug).await;
    let student = app.create_user("student@example.com", true, false).await;
    let coach = app.create_user("coach@example.com", true, false).await;
    app.add_membership(club.id, student.id, member_role.id)
        .await;
    app.add_membership(club.id, coach.id, member_role.id).await;
    support::make_instructor(&app, coach.id, "E3").await;
    support::set_preparing(&app, student.id, certification).await;
    let coach_token = app.session_for(coach.id).await;
    (app, root_token, coach_token, student.id, skill_ids)
}

fn validation_path(slug: &str, student: Uuid, skill: Uuid) -> String {
    format!("/api/v1/clubs/{slug}/members/{student}/skill-validations/{skill}")
}

#[tokio::test]
async fn validation_flow_updates_progress_and_history() {
    let (app, root_token, coach_token, student, skills) = setup("flow", "N2").await;

    // vu → in_progress.
    let (status, view) = app
        .request(
            Method::PUT,
            &validation_path("flow", student, skills[0]),
            Some(&coach_token),
            Some(json!({ "stage": "vu", "notes": "  bon départ  " })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["progress"]["total"], 2);
    assert_eq!(view["progress"]["in_progress"], 1);
    assert_eq!(view["progress"]["validated"], 0);
    assert_eq!(view["progress"]["percentage"], 0);
    let skill_view = view["domains"][0]["modules"][0]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == skills[0].to_string())
        .unwrap()
        .clone();
    assert_eq!(skill_view["validation"]["stage"], "vu");
    assert_eq!(skill_view["validation"]["notes"], "bon départ");
    assert_eq!(
        skill_view["validation"]["validated_by"]["last_name"],
        "User"
    );

    // Jumping straight to the final stage is allowed (Fosse parity).
    let (status, view) = app
        .request(
            Method::PUT,
            &validation_path("flow", student, skills[1]),
            Some(&coach_token),
            Some(json!({ "stage": "valide" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["progress"]["validated"], 1);
    assert_eq!(view["progress"]["percentage"], 50);

    // Advancing skill 0 to the final stage: 100 %.
    let (status, view) = app
        .request(
            Method::PUT,
            &validation_path("flow", student, skills[0]),
            Some(&coach_token),
            Some(json!({ "stage": "valide" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["progress"]["percentage"], 100);

    // History: three validated events, immutable, newest first.
    let (status, events) = app
        .request(
            Method::GET,
            &format!("/api/v1/competencies/validation-events?user_id={student}"),
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{events}");
    assert_eq!(events["total"], 3);
    // Pagination regression (review HIGH-1): page params must parse.
    let (status, paged) = app
        .request(
            Method::GET,
            &format!("/api/v1/competencies/validation-events?user_id={student}&page=2&per_page=2"),
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{paged}");
    assert_eq!(paged["page"], 2);
    assert_eq!(paged["items"].as_array().unwrap().len(), 1);
    assert!(events["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e["action"] == "validated" && e["certification"] == "N2"));
    // The first write's previous_stage is null, the upgrade records vu.
    let for_skill0: Vec<_> = events["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["skill_id"] == skills[0].to_string())
        .collect();
    assert_eq!(for_skill0[0]["stage"], "valide");
    assert_eq!(for_skill0[0]["previous_stage"], "vu");
    assert_eq!(for_skill0[1]["stage"], "vu");
    assert_eq!(for_skill0[1]["previous_stage"], serde_json::Value::Null);
}

#[tokio::test]
async fn dates_and_notes_are_validated() {
    let (app, _, coach_token, student, skills) = setup("dates", "N2").await;
    let path = validation_path("dates", student, skills[0]);

    // Future date: refused.
    let future = chrono::Utc::now().date_naive() + chrono::Duration::days(2);
    let (status, _) = app
        .request(
            Method::PUT,
            &path,
            Some(&coach_token),
            Some(json!({ "stage": "vu", "validated_at": future.to_string() })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Backdating: allowed (paper reality).
    let (status, view) = app
        .request(
            Method::PUT,
            &path,
            Some(&coach_token),
            Some(json!({ "stage": "vu", "validated_at": "2026-01-15" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");

    // Unknown stage code.
    let (status, _) = app
        .request(
            Method::PUT,
            &path,
            Some(&coach_token),
            Some(json!({ "stage": "valide_mer" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Oversized notes.
    let (status, _) = app
        .request(
            Method::PUT,
            &path,
            Some(&coach_token),
            Some(json!({ "stage": "vu", "notes": "x".repeat(2001) })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn me_view_defaults_to_the_prepared_level() {
    let (app, _, coach_token, student, skills) = setup("meview", "N2").await;
    let student_token = app.session_for(student).await;

    let (status, _) = app
        .request(
            Method::PUT,
            &validation_path("meview", student, skills[0]),
            Some(&coach_token),
            Some(json!({ "stage": "valide" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // No parameter: the prepared level drives the view.
    let (status, view) = app
        .request(
            Method::GET,
            "/api/v1/me/competencies",
            Some(&student_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["level"]["certification"], "N2");
    assert_eq!(view["level"]["sea_validation"], true);
    assert_eq!(view["progress"]["validated"], 1);

    // A user preparing nothing gets a 422 (the UI shows its empty state).
    let idle = app.create_user("idle@example.com", true, false).await;
    let idle_token = app.session_for(idle.id).await;
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/me/competencies",
            Some(&idle_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Explicit parameter works for any level.
    let (status, view) = app
        .request(
            Method::GET,
            "/api/v1/me/competencies?certification=N1",
            Some(&idle_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["domains"].as_array().unwrap().len(), 0);
}
