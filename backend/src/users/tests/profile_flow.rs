//! End-to-end profile flows over HTTP: incomplete profile, CACI ownership,
//! derived prerogatives (full persona), stamp gating, members list badges.

use axum::http::{Method, StatusCode};
use base64::Engine;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::test_support::TestApp;
use crate::users::entities::user_certifications;

struct Ctx {
    app: TestApp,
    user_id: Uuid,
    user_token: String,
    admin_token: String,
}

async fn setup(slug: &str) -> Ctx {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club(slug).await;
    let user = app.create_user("diver@example.com", true, false).await;
    let admin = app.create_user("staff@example.com", true, false).await;
    app.add_membership(club.id, user.id, member_role.id).await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    Ctx {
        user_id: user.id,
        user_token: app.session_for(user.id).await,
        admin_token: app.session_for(admin.id).await,
        app,
    }
}

fn full_profile(birthdate: Option<&str>) -> Value {
    json!({
        "birthdate": birthdate,
        "phone": "0600000000",
        "licence_number": "A-01-000000",
        "instructor_number": null,
        "medical_certificate_valid_until": null,
        "gear_regulator": true,
        "gear_stab": true,
        "gear_stab_size": "M",
        "gear_nitrox": false,
        "gear_second_regulator": false,
        "preparing_certification": null
    })
}

#[tokio::test]
async fn missing_birthdate_flags_profile_incomplete() {
    let ctx = setup("flow-a").await;
    let (status, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["derived"]["incomplete"], true);
    // Gear defaults: club divers usually own regulator and stab.
    assert_eq!(body["profile"]["gear_regulator"], true);

    // Setting the birthdate completes it.
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["derived"]["incomplete"], false);
    assert!(body["derived"]["age"].as_u64().unwrap() >= 36);
}

#[tokio::test]
async fn caci_date_is_staff_only() {
    let ctx = setup("flow-b").await;
    // Self PUT tries to smuggle a CACI date → silently kept unset.
    let mut payload = full_profile(Some("1990-05-10"));
    payload["medical_certificate_valid_until"] = json!("2030-01-01");
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["profile"]["medical_certificate_valid_until"],
        Value::Null
    );

    // Staff records an EXPIRED CACI through the club route.
    let mut payload = full_profile(Some("1990-05-10"));
    payload["medical_certificate_valid_until"] = json!("2024-01-01");
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/flow-b/members/{}/profile", ctx.user_id),
            Some(&ctx.admin_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["profile"]["medical_certificate_valid_until"],
        "2024-01-01"
    );
    assert_eq!(body["derived"]["medical_expired"], true);
}

#[tokio::test]
async fn derived_prerogatives_follow_the_regulation() {
    let ctx = setup("flow-c").await;
    ctx.app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;
    // Adult N2 + Nitrox (staff-recorded — the only entry path).
    for code in ["N2", "NITROX"] {
        let (status, _) = ctx
            .app
            .request(
                Method::POST,
                &format!(
                    "/api/v1/clubs/flow-c/members/{}/certifications",
                    ctx.user_id
                ),
                Some(&ctx.admin_token),
                Some(json!({ "certification": code })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            None,
        )
        .await;
    let derived = &body["derived"];
    let codes: Vec<&str> = derived["aptitudes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["code"].as_str().unwrap())
        .collect();
    // N2 confers PA20 + PE40, exercisable as an adult.
    assert!(codes.contains(&"PA20"), "aptitudes: {codes:?}");
    assert!(codes.contains(&"PE40"), "aptitudes: {codes:?}");
    assert_eq!(derived["gp"], false);
    assert_eq!(derived["teaching"], Value::Null);
    assert_eq!(derived["dp_exploration"], false);
    assert_eq!(derived["dp_trimix_shallow"], false);
    // Nitrox (base) → 40 % O2 (MFT Nitrox).
    assert_eq!(derived["max_o2_percent"], 40);
    // Progression: an adult N2 sees the N3 brevet (RIFAP is a delivery
    // condition) — its PA40/PE60 modules are absorbed. Initiateur shows too
    // (PA20 aptitude held, RIFAP at exam).
    let next: Vec<&str> = derived["eligible_next"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert!(next.contains(&"N3"), "eligible_next: {next:?}");
    assert!(next.contains(&"INITIATEUR"), "eligible_next: {next:?}");
    assert!(!next.contains(&"PA40"), "absorbed by N3: {next:?}");
    assert!(!next.contains(&"PE60"), "absorbed by N3: {next:?}");
    assert!(!next.contains(&"N2"), "already held: {next:?}");
    // Covered/proved by the N2 → never proposed (product rule 2026-08-18).
    for covered in ["PE12", "PA12", "PE40", "PA20", "N1"] {
        assert!(
            !next.contains(&covered),
            "{covered} covered by N2: {next:?}"
        );
    }
    // PESH gated to the handisub pathway.
    assert!(!next.contains(&"PESH6"), "handisub gate: {next:?}");
    // The preparing picker list tolerates pending entry requirements (N3).
    let preparable: Vec<&str> = derived["preparable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert!(preparable.contains(&"N3"), "preparable: {preparable:?}");
    assert!(!preparable.contains(&"PE40"), "preparable: {preparable:?}");
    assert_eq!(derived["regulation_snapshot"], ffessm::REGULATION_SNAPSHOT);

    // Preparing a covered certification is refused server-side.
    let mut payload = full_profile(Some("1990-05-10"));
    payload["preparing_certification"] = json!("PE40");
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Flagging the handisub pathway opens the PESH suggestions.
    let mut payload = full_profile(Some("1990-05-10"));
    payload["handisub"] = json!(true);
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["derived"]["eligible_next"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["code"] == "PESH6"));
}

#[tokio::test]
async fn stagiaire_mf1_status_grants_teaching_but_not_dp() {
    let ctx = setup("flow-d").await;
    ctx.app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;
    // Staff records N4 (prerequisite in real life) then the MF1 livret.
    ctx.app
        .request(
            Method::POST,
            &format!(
                "/api/v1/clubs/flow-d/members/{}/certifications",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N4", "validated": true })),
        )
        .await;
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/flow-d/members/{}/training-status",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "stagiaire_mf1_valid_until": "2029-01-01" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let derived = &body["derived"];
    // E2 while the livret is valid, but never DP (MFT MF1).
    assert_eq!(derived["teaching"], "E2");
    assert_eq!(derived["gp"], true);
    assert_eq!(derived["dp_pool"], false);
    assert_eq!(derived["dp_teaching"], false);

    // An EXPIRED livret grants nothing.
    let (_, body) = ctx
        .app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/flow-d/members/{}/training-status",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "stagiaire_mf1_valid_until": "2020-01-01" })),
        )
        .await;
    assert_eq!(body["derived"]["teaching"], Value::Null);

    // Removing the status drops the grade.
    let (_, body) = ctx
        .app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/flow-d/members/{}/training-status",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "stagiaire_mf1_valid_until": null })),
        )
        .await;
    assert_eq!(body["derived"]["teaching"], Value::Null);
}

fn png_base64() -> String {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&[0u8; 64]);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test]
async fn stamp_is_gated_to_certified_e2_plus() {
    let ctx = setup("flow-e").await;
    ctx.app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;

    // A plain N2 diver cannot upload a stamp.
    ctx.app
        .request(
            Method::POST,
            &format!(
                "/api/v1/clubs/flow-e/members/{}/certifications",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N2" })),
        )
        .await;
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/stamp",
            Some(&ctx.user_token),
            Some(json!({ "image_base64": png_base64() })),
        )
        .await;
    // Authorization rule (E2+ certified), not a payload problem.
    assert_eq!(status, StatusCode::FORBIDDEN);

    // MF1 (certified E3) → upload allowed; bad payloads still refused.
    ctx.app
        .request(
            Method::POST,
            &format!(
                "/api/v1/clubs/flow-e/members/{}/certifications",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "MF1", "validated": true })),
        )
        .await;
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/stamp",
            Some(&ctx.user_token),
            Some(json!({ "image_base64":
                base64::engine::general_purpose::STANDARD.encode(b"GIF89a not an allowed format") })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/stamp",
            Some(&ctx.user_token),
            Some(json!({ "image_base64": png_base64() })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Readable by self and by club staff; flagged in the profile payload.
    let (status, _) = ctx
        .app
        .request(Method::GET, "/api/v1/me/stamp", Some(&ctx.user_token), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = ctx
        .app
        .request(
            Method::GET,
            &format!("/api/v1/clubs/flow-e/members/{}/stamp", ctx.user_id),
            Some(&ctx.admin_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            None,
        )
        .await;
    assert_eq!(body["has_stamp"], true);
    // MF1 = E3: DP trimix in the LIMITED zone, not all zones (F1).
    assert_eq!(body["derived"]["dp_trimix_shallow"], true);
    assert_eq!(body["derived"]["dp_trimix"], false);
    assert_eq!(body["derived"]["dp_teaching"], true);

    // Oversize image (> 512 KiB) is refused even for an instructor.
    let mut big = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    big.resize(512 * 1024 + 1, 0);
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/stamp",
            Some(&ctx.user_token),
            Some(json!({
                "image_base64": base64::engine::general_purpose::STANDARD.encode(&big)
            })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn unknown_certification_code_in_database_is_skipped_not_fatal() {
    let ctx = setup("flow-h").await;
    ctx.app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;
    // Simulate legacy/corrupt data written outside the API.
    let now = crate::now();
    user_certifications::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(ctx.user_id),
        certification: Set("PADI-OW".to_string()),
        obtained_on: Set(None),
        provenance: Set("imported".to_string()),
        recorded_by_user_id: Set(None),
        recorded_in_club_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&ctx.app.state.db)
    .await
    .unwrap();

    let (status, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            None,
        )
        .await;
    // Never a 500: the row is listed (label falls back to the raw code) but
    // ignored by every derivation.
    assert_eq!(status, StatusCode::OK);
    let row = body["certifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "PADI-OW")
        .unwrap();
    assert_eq!(row["label"], "PADI-OW");
    assert_eq!(body["derived"]["incomplete"], false);
}

#[tokio::test]
async fn certification_catalog_is_authenticated_and_carries_coverage() {
    let ctx = setup("flow-k").await;
    // Unauthenticated → 401.
    let (status, _) = ctx
        .app
        .request(Method::GET, "/api/v1/catalog/certifications", None, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Any authenticated member reads the catalog (static domain data).
    let (status, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/catalog/certifications",
            Some(&ctx.user_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().unwrap();
    assert_eq!(entries.len(), ffessm::Certification::ALL.len());
    let n2 = entries.iter().find(|e| e["code"] == "N2").unwrap();
    assert_eq!(n2["label"], "Niveau 2");
    let covers: Vec<&str> = n2["covers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(covers.contains(&"N1"), "covers: {covers:?}");
    assert!(covers.contains(&"PE40"), "covers: {covers:?}");
}

#[tokio::test]
async fn members_list_is_paginated() {
    let ctx = setup("flow-i").await;
    // setup() created 2 members (diver + staff); add a third.
    let extra = ctx.app.create_user("third@example.com", true, false).await;
    let (_, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/clubs/flow-i/members?per_page=2",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    // `extra` is not a member yet: 2 members, one page.
    assert_eq!(body["total"], 2);

    let (_, roles) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/clubs/flow-i/roles",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    let member_role = roles
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Membre")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    ctx.app
        .request(
            Method::POST,
            "/api/v1/clubs/flow-i/members",
            Some(&ctx.admin_token),
            Some(json!({ "first_name": "Test", "last_name": "User", "email": "third@example.com", "role_id": member_role })),
        )
        .await;
    let _ = extra;

    let (_, page1) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/clubs/flow-i/members?per_page=2&page=1",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    let (_, page2) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/clubs/flow-i/members?per_page=2&page=2",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    assert_eq!(page1["total"], 3);
    assert_eq!(page1["items"].as_array().unwrap().len(), 2);
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);
    assert_eq!(page2["page"], 2);
}

#[tokio::test]
async fn members_list_carries_diving_badges() {
    let ctx = setup("flow-f").await;
    ctx.app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;
    for (code, validated) in [("N2", true), ("RIFAP", true)] {
        ctx.app
            .request(
                Method::POST,
                &format!(
                    "/api/v1/clubs/flow-f/members/{}/certifications",
                    ctx.user_id
                ),
                Some(&ctx.admin_token),
                Some(json!({ "certification": code, "validated": validated })),
            )
            .await;
    }
    let (status, body) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/clubs/flow-f/members",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let diver = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["user"]["email"] == "diver@example.com")
        .unwrap();
    assert_eq!(diver["badges"]["diver"], "N2");
    assert_eq!(diver["badges"]["has_rifap"], true);
    assert_eq!(diver["badges"]["incomplete"], false);
    assert_eq!(diver["badges"]["medical_expired"], Value::Null);
    // The admin never filled a profile → flagged incomplete.
    let staff = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["user"]["email"] == "staff@example.com")
        .unwrap();
    assert_eq!(staff["badges"]["incomplete"], true);
}

#[tokio::test]
async fn instructor_number_requires_a_teaching_grade() {
    let ctx = setup("flow-j").await;

    // A plain member (birthdate set, no teaching grade) cannot set a number.
    let mut payload = full_profile(Some("1990-05-10"));
    payload["instructor_number"] = json!("E-12345");
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Staff validates MF1 (certified E3) → the same PUT is now accepted.
    ctx.app
        .request(
            Method::POST,
            &format!(
                "/api/v1/clubs/flow-j/members/{}/certifications",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "MF1", "validated": true })),
        )
        .await;
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["profile"]["instructor_number"], "E-12345");

    // Second member: the only teaching source is the stagiaire-MF1 status
    // (dated E2, per ADR-0007/Q5) — the number is accepted too.
    let second = ctx.app.create_user("second@example.com", true, false).await;
    let second_token = ctx.app.session_for(second.id).await;
    let (_, roles) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/clubs/flow-j/roles",
            Some(&ctx.admin_token),
            None,
        )
        .await;
    let member_role = roles
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Membre")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    ctx.app
        .request(
            Method::POST,
            "/api/v1/clubs/flow-j/members",
            Some(&ctx.admin_token),
            Some(json!({ "first_name": "Test", "last_name": "User", "email": "second@example.com", "role_id": member_role })),
        )
        .await;
    ctx.app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&second_token),
            Some(full_profile(Some("1995-03-01"))),
        )
        .await;
    ctx.app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/flow-j/members/{}/certifications", second.id),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N4", "validated": true })),
        )
        .await;
    ctx.app
        .request(
            Method::PUT,
            &format!("/api/v1/clubs/flow-j/members/{}/training-status", second.id),
            Some(&ctx.admin_token),
            Some(json!({ "stagiaire_mf1_valid_until": "2030-01-01" })),
        )
        .await;
    let mut payload = full_profile(Some("1995-03-01"));
    payload["instructor_number"] = json!("S-00001");
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&second_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["profile"]["instructor_number"], "S-00001");

    // Clearing the field (null) is always allowed.
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(full_profile(Some("1990-05-10"))),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["profile"]["instructor_number"], Value::Null);
}

#[tokio::test]
async fn preparing_certification_is_validated_and_reports_issues() {
    let ctx = setup("flow-g").await;
    let mut payload = full_profile(Some("1990-05-10"));

    // Externally delivered title → refused.
    payload["preparing_certification"] = json!("BEES1");
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Preparing N3 without N2 → stored, with eligibility issues reported.
    payload["preparing_certification"] = json!("N3");
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body["derived"]["preparing_issues"]
        .as_array()
        .unwrap()
        .is_empty());

    // Already-held certification → refused.
    ctx.app
        .request(
            Method::POST,
            &format!(
                "/api/v1/clubs/flow-g/members/{}/certifications",
                ctx.user_id
            ),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N3" })),
        )
        .await;
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
