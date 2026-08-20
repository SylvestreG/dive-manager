//! Level documents: upload validation, real-global-admin gating (Fosse had
//! ZERO server-side checks here), stamp positions, filled-PDF generation.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use crate::competencies::services::pdf::tiny_pdf;
use crate::competencies::tests::support;
use crate::test_support::TestApp;

fn pdf_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(tiny_pdf())
}

/// Raw request helper: the JSON helper cannot read PDF bodies/headers.
async fn raw_get(app: &TestApp, path: &str, token: &str) -> (StatusCode, String, Vec<u8>) {
    let request = Request::builder()
        .method(Method::GET)
        .uri(path)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("request");
    let response = app.router.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, content_type, bytes)
}

#[tokio::test]
async fn upload_is_validated_and_admin_only() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let path = "/api/v1/competencies/levels/N2/document";

    // Plain authenticated user: 403 (THE Fosse hole).
    let member = app.create_user("member@example.com", true, false).await;
    let member_token = app.session_for(member.id).await;
    for method in [Method::PUT, Method::DELETE, Method::GET] {
        let body = (method == Method::PUT)
            .then(|| json!({ "file_name": "n2.pdf", "pdf_base64": pdf_base64() }));
        let (status, _) = app
            .request(method.clone(), path, Some(&member_token), body)
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method}");
    }

    // Not a PDF.
    let (status, _) = app
        .request(
            Method::PUT,
            path,
            Some(&root_token),
            Some(json!({
                "file_name": "x.pdf",
                "pdf_base64": base64::engine::general_purpose::STANDARD.encode(b"hello")
            })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Bad base64.
    let (status, _) = app
        .request(
            Method::PUT,
            path,
            Some(&root_token),
            Some(json!({ "file_name": "x.pdf", "pdf_base64": "!!!" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Valid upload; the filename is sanitized for Content-Disposition.
    let (status, info) = app
        .request(
            Method::PUT,
            path,
            Some(&root_token),
            Some(json!({ "file_name": "N2 \"guide\"\r\n.pdf", "pdf_base64": pdf_base64() })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{info}");
    assert_eq!(info["page_count"], 1);
    assert!(!info["file_name"].as_str().unwrap().contains('"'));
    assert!(!info["file_name"].as_str().unwrap().contains('\n'));

    // Download round-trips the bytes.
    let (status, content_type, bytes) = raw_get(&app, path, &root_token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/pdf");
    assert!(bytes.starts_with(b"%PDF"));

    // The levels list now reports the document — without loading the blob.
    let (status, levels) = app
        .request(
            Method::GET,
            "/api/v1/competencies/levels",
            Some(&member_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let n2 = levels
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["certification"] == "N2")
        .expect("N2 level")
        .clone();
    assert_eq!(n2["has_document"], true);
    assert_eq!(n2["page_count"], 1);
}

#[tokio::test]
async fn positions_are_scoped_to_the_level_and_its_pages() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, n2_skills) = support::build_tree(&app, &root_token, "N2").await;
    let (_, _, n1_skills) = support::build_tree(&app, &root_token, "N1").await;

    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/competencies/levels/N2/document",
            Some(&root_token),
            Some(json!({ "file_name": "n2.pdf", "pdf_base64": pdf_base64() })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let position = json!({ "page": 1, "x": 100.0, "y": 500.0, "width": 120.0, "height": 24.0 });

    // A skill from another level's tree is refused.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/competencies/levels/N2/positions/{}", n1_skills[0]),
            Some(&root_token),
            Some(position.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Page out of range (the template has 1 page).
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/competencies/levels/N2/positions/{}", n2_skills[0]),
            Some(&root_token),
            Some(json!({ "page": 3, "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Upsert, list, move, delete.
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/competencies/levels/N2/positions/{}", n2_skills[0]),
            Some(&root_token),
            Some(position.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::PUT,
            &format!("/api/v1/competencies/levels/N2/positions/{}", n2_skills[0]),
            Some(&root_token),
            Some(json!({ "page": 1, "x": 200.0, "y": 300.0, "width": 100.0, "height": 20.0 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, positions) = app
        .request(
            Method::GET,
            "/api/v1/competencies/levels/N2/positions",
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{positions}");
    let list = positions.as_array().unwrap();
    assert_eq!(list.len(), 1, "upsert must not duplicate");
    assert_eq!(list[0]["x"], 200.0);
    assert_eq!(list[0]["font_size"], 8.0);
    assert_eq!(list[0]["out_of_range"], false);

    // Batch is transactional: one bad item rolls everything back.
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/competencies/levels/N2/positions",
            Some(&root_token),
            Some(json!([
                { "skill_id": n2_skills[1], "page": 1, "x": 10.0, "y": 10.0, "width": 50.0, "height": 15.0 },
                { "skill_id": n2_skills[0], "page": 99, "x": 10.0, "y": 10.0, "width": 50.0, "height": 15.0 }
            ])),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, positions) = app
        .request(
            Method::GET,
            "/api/v1/competencies/levels/N2/positions",
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(positions.as_array().unwrap().len(), 1, "batch rolled back");

    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/competencies/levels/N2/positions/{}", n2_skills[0]),
            Some(&root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn generation_stamps_final_stages_only() {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, skills) = support::build_tree(&app, &root_token, "N2").await;
    let (status, _) = app
        .request(
            Method::PUT,
            "/api/v1/competencies/levels/N2/document",
            Some(&root_token),
            Some(json!({ "file_name": "n2.pdf", "pdf_base64": pdf_base64() })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    for skill in &skills {
        let (status, _) = app
            .request(
                Method::PUT,
                &format!("/api/v1/competencies/levels/N2/positions/{skill}"),
                Some(&root_token),
                Some(json!({ "page": 1, "x": 100.0, "y": 500.0, "width": 120.0, "height": 24.0 })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (club, _, member_role) = app.create_club("gen").await;
    let diver = app.create_user("diver@example.com", true, false).await;
    let coach = app.create_user("coach@example.com", true, false).await;
    app.add_membership(club.id, diver.id, member_role.id).await;
    app.add_membership(club.id, coach.id, member_role.id).await;
    support::make_instructor(&app, coach.id, "E3").await;
    support::set_preparing(&app, diver.id, "N2").await;
    let coach_token = app.session_for(coach.id).await;
    let diver_token = app.session_for(diver.id).await;

    // No document for N1 → 404 for the diver.
    let (status, _, _) = raw_get(
        &app,
        "/api/v1/me/competencies/document?certification=N1",
        &diver_token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // One skill final, one in progress.
    for (skill, stage) in [(skills[0], "valide"), (skills[1], "vu")] {
        let (status, _) = app
            .request(
                Method::PUT,
                &format!(
                    "/api/v1/clubs/gen/members/{}/skill-validations/{skill}",
                    diver.id
                ),
                Some(&coach_token),
                Some(json!({ "stage": stage })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    // The diver downloads their own PDF (defaults to the prepared level).
    let (status, content_type, bytes) =
        raw_get(&app, "/api/v1/me/competencies/document", &diver_token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/pdf");
    assert!(bytes.starts_with(b"%PDF"));
    // Exactly ONE stamp: only the final stage is stamped. The coach line
    // ("Test User (E3)") appears once in the overlay stream.
    let document = lopdf::Document::load_mem(&bytes).expect("filled pdf parses");
    let page_id = *document.get_pages().get(&1).expect("page 1");
    let content = document.get_page_content(page_id).expect("content");
    let text = String::from_utf8_lossy(&content);
    // Parentheses are escaped inside PDF strings: match the name alone.
    assert_eq!(text.matches("Test User").count(), 1, "{text}");
    assert!(text.contains("\\(E3\\)"), "{text}");

    // Staff-side generation (the gap Fosse never filled).
    let (status, content_type, _) = raw_get(
        &app,
        &format!(
            "/api/v1/clubs/gen/members/{}/competencies/document?certification=N2",
            diver.id
        ),
        &coach_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/pdf");

    // A plain member of the club cannot pull someone else's document.
    let peer = app.create_user("peer@example.com", true, false).await;
    app.add_membership(club.id, peer.id, member_role.id).await;
    let peer_token = app.session_for(peer.id).await;
    let (status, _, _) = raw_get(
        &app,
        &format!(
            "/api/v1/clubs/gen/members/{}/competencies/document?certification=N2",
            diver.id
        ),
        &peer_token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
