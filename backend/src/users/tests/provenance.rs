//! Provenance matrix (business rule 2): recorded = imported < validated; a
//! write may only keep-or-raise. Certifications are staff-entered ONLY
//! (product decision 2026-08-18): members have no write endpoint at all.

use axum::http::{Method, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::test_support::TestApp;

struct Ctx {
    app: TestApp,
    user_id: Uuid,
    user_token: String,
    admin_token: String,
}

async fn setup() -> Ctx {
    let app = TestApp::new().await;
    let (club, admin_role, member_role) = app.create_club("prov").await;
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

fn cert<'a>(profile: &'a Value, code: &str) -> Option<&'a Value> {
    profile["certifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == code)
}

#[tokio::test]
async fn members_have_no_certification_write_endpoint() {
    let ctx = setup().await;
    // The self-declaration routes do not exist (staff-entered only).
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            "/api/v1/me/certifications",
            Some(&ctx.user_token),
            Some(json!({ "certification": "N1" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = ctx
        .app
        .request(
            Method::DELETE,
            &format!("/api/v1/me/certifications/{}", Uuid::now_v7()),
            Some(&ctx.user_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Members cannot write through the club route either (no permission).
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/prov/members/{}/certifications", ctx.user_id),
            Some(&ctx.user_token),
            Some(json!({ "certification": "N1" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn provenance_only_raises_and_staff_can_correct() {
    let ctx = setup().await;
    let member_certs = format!("/api/v1/clubs/prov/members/{}/certifications", ctx.user_id);

    // 1. Staff records → recorded; the member SEES it on their profile.
    let (status, body) = ctx
        .app
        .request(
            Method::POST,
            &member_certs,
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N1", "obtained_on": "2020-06-01" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cert(&body, "N1").unwrap()["provenance"], "recorded");
    let (_, me_profile) = ctx
        .app
        .request(
            Method::GET,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            None,
        )
        .await;
    assert_eq!(cert(&me_profile, "N1").unwrap()["provenance"], "recorded");

    // 2. Staff validates → strongest rank.
    let (status, body) = ctx
        .app
        .request(
            Method::POST,
            &member_certs,
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N1", "validated": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let n1 = cert(&body, "N1").unwrap();
    assert_eq!(n1["provenance"], "validated");
    let n1_id = n1["id"].as_str().unwrap().to_string();

    // 3. Recording over validated would lower the rank → 409.
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            &member_certs,
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N1" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 4. A club admin CAN delete a validated row (product decision:
    //    admins correct mistakes).
    let (status, body) = ctx
        .app
        .request(
            Method::DELETE,
            &format!("{member_certs}/{n1_id}"),
            Some(&ctx.admin_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(cert(&body, "N1").is_none());
}

#[tokio::test]
async fn obtaining_a_certification_clears_its_preparation() {
    let ctx = setup().await;
    // Profile prepares N2 (not held yet).
    let (status, body) = ctx
        .app
        .request(
            Method::PUT,
            "/api/v1/me/profile",
            Some(&ctx.user_token),
            Some(json!({
                "birthdate": "1990-05-10", "phone": null, "licence_number": null,
                "instructor_number": null, "medical_certificate_valid_until": null,
                "gear_regulator": true, "gear_stab": true, "gear_stab_size": null,
                "gear_nitrox": false, "gear_second_regulator": false,
                "preparing_certification": "N2"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["profile"]["preparing_certification"], "N2");

    // Staff records N2 → the stale preparation is cleared in the same txn.
    let (status, body) = ctx
        .app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/prov/members/{}/certifications", ctx.user_id),
            Some(&ctx.admin_token),
            Some(json!({ "certification": "N2", "validated": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["profile"]["preparing_certification"], Value::Null);
    assert!(cert(&body, "N2").is_some());
}

#[tokio::test]
async fn certification_inputs_are_validated() {
    let ctx = setup().await;
    let member_certs = format!("/api/v1/clubs/prov/members/{}/certifications", ctx.user_id);
    // Unknown code → 422 (never stored, business rule 4).
    let (status, _) = ctx
        .app
        .request(
            Method::POST,
            &member_certs,
            Some(&ctx.admin_token),
            Some(json!({ "certification": "PADI-OW" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    // Future or absurd obtained_on → 422.
    for date in ["2100-01-01", "1850-01-01"] {
        let (status, _) = ctx
            .app
            .request(
                Method::POST,
                &member_certs,
                Some(&ctx.admin_token),
                Some(json!({ "certification": "N1", "obtained_on": date })),
            )
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{date}");
    }
}
