//! Shared fixtures for the competencies tests: personas (instructors of each
//! grade, students, admins) and a small N2 tree.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::test_support::TestApp;

/// Records a certification row directly (provenance recorded) — the test
/// shortcut equivalent of the staff endpoint.
pub async fn record_certification(app: &TestApp, user_id: Uuid, code: &str) {
    use crate::users::entities::user_certifications;
    use sea_orm::{ActiveModelTrait, Set};
    let created = crate::now();
    user_certifications::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        certification: Set(code.to_string()),
        obtained_on: Set(None),
        provenance: Set("recorded".to_string()),
        recorded_by_user_id: Set(None),
        recorded_in_club_id: Set(None),
        created_at: Set(created),
        updated_at: Set(created),
    }
    .insert(&app.state.db)
    .await
    .expect("certification");
}

/// Gives the user a derived teaching grade via certifications:
/// E1 = Initiateur; E2 = Initiateur + N4; E3 = MF1; E4 = MF2.
pub async fn make_instructor(app: &TestApp, user_id: Uuid, grade: &str) {
    match grade {
        "E1" => record_certification(app, user_id, "INITIATEUR").await,
        "E2" => {
            record_certification(app, user_id, "INITIATEUR").await;
            record_certification(app, user_id, "N4").await;
        }
        "E3" => record_certification(app, user_id, "MF1").await,
        "E4" => record_certification(app, user_id, "MF2").await,
        other => panic!("unknown grade {other}"),
    }
}

/// Sets the profile's preparing certification directly.
pub async fn set_preparing(app: &TestApp, user_id: Uuid, code: &str) {
    use crate::users::entities::user_profiles;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let existing = user_profiles::Entity::find_by_id(user_id)
        .one(&app.state.db)
        .await
        .expect("profile query");
    let created = crate::now();
    match existing {
        Some(profile) => {
            let mut active: user_profiles::ActiveModel = profile.into();
            active.preparing_certification = Set(Some(code.to_string()));
            active.updated_at = Set(created);
            active.update(&app.state.db).await.expect("profile update");
        }
        None => {
            user_profiles::ActiveModel {
                user_id: Set(user_id),
                birthdate: Set(None),
                phone: Set(None),
                licence_number: Set(None),
                instructor_number: Set(None),
                medical_certificate_valid_until: Set(None),
                gear_regulator: Set(true),
                gear_stab: Set(true),
                gear_stab_size: Set(None),
                gear_nitrox: Set(false),
                gear_second_regulator: Set(false),
                preparing_certification: Set(Some(code.to_string())),
                handisub: Set(false),
                created_at: Set(created),
                updated_at: Set(created),
            }
            .insert(&app.state.db)
            .await
            .expect("profile insert");
        }
    }
}

/// A minimal tree: one domain / one module / two skills on `certification`.
/// Returns (domain_id, module_id, skill_ids).
pub async fn build_tree(
    app: &TestApp,
    admin_token: &str,
    certification: &str,
) -> (Uuid, Uuid, Vec<Uuid>) {
    let (status, domain) = app
        .request(
            axum::http::Method::POST,
            "/api/v1/competencies/domains",
            Some(admin_token),
            Some(json!({ "certification": certification, "name": format!("COMMUNES {certification}") })),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{domain}");
    let domain_id: Uuid = uuid(&domain["id"]);

    let (status, module) = app
        .request(
            axum::http::Method::POST,
            "/api/v1/competencies/modules",
            Some(admin_token),
            Some(json!({ "domain_id": domain_id, "name": "Technique" })),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{module}");
    let module_id: Uuid = uuid(&module["id"]);

    let mut skill_ids = Vec::new();
    for name in ["Vidage de masque", "Remontée assistée"] {
        let (status, skill) = app
            .request(
                axum::http::Method::POST,
                "/api/v1/competencies/skills",
                Some(admin_token),
                Some(json!({ "module_id": module_id, "name": name })),
            )
            .await;
        assert_eq!(status, axum::http::StatusCode::OK, "{skill}");
        skill_ids.push(uuid(&skill["id"]));
    }
    (domain_id, module_id, skill_ids)
}

pub fn uuid(value: &Value) -> Uuid {
    value
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("uuid in response")
}
