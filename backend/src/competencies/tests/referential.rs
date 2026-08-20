//! The global referential: reads for everyone authenticated, writes for
//! real global admins only, cascade-impact preflight, uniqueness.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::competencies::tests::support;
use crate::test_support::TestApp;

#[tokio::test]
async fn tree_reads_are_authenticated_and_shaped() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    support::build_tree(&app, &root_token, "N2").await;

    // Unauthenticated: nothing.
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/competencies/tree?certification=N2",
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Any authenticated user reads the referential.
    let member = app.create_user("member@example.com", true, false).await;
    let member_token = app.session_for(member.id).await;
    let (status, tree) = app
        .request(
            Method::GET,
            "/api/v1/competencies/tree?certification=N2",
            Some(&member_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(tree["level"]["certification"], "N2");
    assert_eq!(tree["level"]["required_validator_grade"], "E2");
    assert_eq!(tree["level"]["sea_validation"], true);
    assert_eq!(
        tree["level"]["stages"],
        json!(["vu", "a_perfectionner", "valide"])
    );
    assert_eq!(tree["domains"].as_array().unwrap().len(), 1);
    let modules = tree["domains"][0]["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0]["skills"].as_array().unwrap().len(), 2);

    // A level without a tree is empty, not an error.
    let (status, empty) = app
        .request(
            Method::GET,
            "/api/v1/competencies/tree?certification=N3",
            Some(&member_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["domains"].as_array().unwrap().len(), 0);

    // Unknown code: 422, not a silent skip (Fosse was fail-open).
    let (status, _) = app
        .request(
            Method::GET,
            "/api/v1/competencies/tree?certification=NX",
            Some(&member_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Levels list reflects the N1-and-below vs above-N1 rules.
    support::build_tree(&app, &root_token, "N1").await;
    let (status, levels) = app
        .request(
            Method::GET,
            "/api/v1/competencies/levels",
            Some(&member_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{levels}");
    let list = levels.as_array().unwrap();
    let level = |code: &str| {
        list.iter()
            .find(|l| l["certification"] == code)
            .unwrap_or_else(|| panic!("level {code} missing"))
            .clone()
    };
    assert_eq!(level("N1")["sea_validation"], false);
    assert_eq!(level("N1")["required_validator_grade"], "E1");
    assert_eq!(level("N2")["domains_count"], 1);
    assert_eq!(level("N2")["has_document"], false);
}

#[tokio::test]
async fn referential_writes_are_real_global_admin_only() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (domain_id, module_id, skill_ids) = support::build_tree(&app, &root_token, "N2").await;

    // A club admin is NOT a platform admin: every write is refused.
    let (club, admin_role, _) = app.create_club("club").await;
    let club_admin = app.create_user("club-admin@example.com", true, false).await;
    app.add_membership(club.id, club_admin.id, admin_role.id)
        .await;
    let club_admin_token = app.session_for(club_admin.id).await;

    let writes: Vec<(Method, String, Option<serde_json::Value>)> = vec![
        (
            Method::POST,
            "/api/v1/competencies/domains".to_string(),
            Some(json!({ "certification": "N2", "name": "X" })),
        ),
        (
            Method::PUT,
            format!("/api/v1/competencies/domains/{domain_id}"),
            Some(json!({ "certification": "N2", "name": "X", "sort_order": 1 })),
        ),
        (
            Method::DELETE,
            format!("/api/v1/competencies/domains/{domain_id}"),
            None,
        ),
        (
            Method::POST,
            "/api/v1/competencies/modules".to_string(),
            Some(json!({ "domain_id": domain_id, "name": "X" })),
        ),
        (
            Method::PUT,
            format!("/api/v1/competencies/modules/{module_id}"),
            Some(json!({ "name": "X", "sort_order": 1 })),
        ),
        (
            Method::DELETE,
            format!("/api/v1/competencies/modules/{module_id}"),
            None,
        ),
        (
            Method::POST,
            "/api/v1/competencies/skills".to_string(),
            Some(json!({ "module_id": module_id, "name": "X" })),
        ),
        (
            Method::PUT,
            format!("/api/v1/competencies/skills/{}", skill_ids[0]),
            Some(json!({ "name": "X", "description": null, "sort_order": 1 })),
        ),
        (
            Method::DELETE,
            format!("/api/v1/competencies/skills/{}", skill_ids[0]),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/competencies/domains/{domain_id}/impact"),
            None,
        ),
        (
            Method::GET,
            "/api/v1/competencies/validation-events".to_string(),
            None,
        ),
    ];
    for (method, path, body) in writes {
        let (status, _) = app
            .request(method.clone(), &path, Some(&club_admin_token), body)
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn duplicate_names_conflict_and_updates_replace() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let token = app.session_for(root.id).await;
    let (domain_id, module_id, skill_ids) = support::build_tree(&app, &token, "N2").await;

    // Same (certification, name): 409 via the unique index.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/competencies/domains",
            Some(&token),
            Some(json!({ "certification": "N2", "name": "COMMUNES N2" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Full-replace update can clear the description.
    let (status, updated) = app
        .request(
            Method::PUT,
            &format!("/api/v1/competencies/skills/{}", skill_ids[0]),
            Some(&token),
            Some(json!({ "name": "Vidage", "description": null, "sort_order": 5 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["description"], serde_json::Value::Null);
    assert_eq!(updated["sort_order"], 5);

    // Moving a domain to another level revalidates the code.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/competencies/domains/{domain_id}"),
            Some(&token),
            Some(json!({ "certification": "NOPE", "name": "COMMUNES", "sort_order": 1 })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Empty names are refused everywhere.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/competencies/modules",
            Some(&token),
            Some(json!({ "domain_id": domain_id, "name": "  " })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let _ = module_id;
}

#[tokio::test]
async fn impact_counts_validations_before_a_cascade_delete() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (domain_id, _, skill_ids) = support::build_tree(&app, &root_token, "N2").await;

    let (club, _, member_role) = app.create_club("impact").await;
    let student = app.create_user("student@example.com", true, false).await;
    let coach = app.create_user("coach@example.com", true, false).await;
    app.add_membership(club.id, student.id, member_role.id)
        .await;
    app.add_membership(club.id, coach.id, member_role.id).await;
    support::make_instructor(&app, coach.id, "E2").await;
    let coach_token = app.session_for(coach.id).await;

    let (status, body) = app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/impact/members/{}/skill-validations/{}",
                student.id, skill_ids[0]
            ),
            Some(&coach_token),
            Some(json!({ "stage": "vu" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, impact) = app
        .request(
            Method::GET,
            &format!("/api/v1/competencies/domains/{domain_id}/impact"),
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(impact["validations_count"], 1);

    // Deleting the domain cascades the validation; the history event stays
    // (denormalized names survive the SET NULL).
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/competencies/domains/{domain_id}"),
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, events) = app
        .request(
            Method::GET,
            &format!(
                "/api/v1/competencies/validation-events?user_id={}",
                student.id
            ),
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{events}");
    assert_eq!(events["total"], 1);
    assert_eq!(events["items"][0]["skill_name"], "Vidage de masque");
    assert_eq!(events["items"][0]["skill_id"], serde_json::Value::Null);
}
