//! Authorization matrix: the validator gate is the DERIVED teaching grade
//! per level (never a permission), impersonation gains no writes, and
//! regressions/deletions are reserved to real global admins.

use axum::http::{Method, StatusCode};
use serde_json::json;
use uuid::Uuid;

use crate::competencies::tests::support;
use crate::test_support::TestApp;

struct Ctx {
    app: TestApp,
    club_slug: &'static str,
    student_id: Uuid,
    skill_ids: Vec<Uuid>,
    root_token: String,
    member_role_id: Uuid,
    club_id: Uuid,
}

/// A club with a student preparing N2 and an N2 tree.
async fn setup(club_slug: &'static str, certification: &str) -> Ctx {
    let app = TestApp::new().await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let (_, _, skill_ids) = support::build_tree(&app, &root_token, certification).await;

    let (club, _, member_role) = app.create_club(club_slug).await;
    let student = app.create_user("student@example.com", true, false).await;
    app.add_membership(club.id, student.id, member_role.id)
        .await;
    support::set_preparing(&app, student.id, certification).await;
    Ctx {
        club_slug,
        student_id: student.id,
        skill_ids,
        root_token,
        member_role_id: member_role.id,
        club_id: club.id,
        app,
    }
}

impl Ctx {
    async fn validator(&self, email: &str, grade: Option<&str>) -> String {
        let user = self.app.create_user(email, true, false).await;
        self.app
            .add_membership(self.club_id, user.id, self.member_role_id)
            .await;
        if let Some(grade) = grade {
            support::make_instructor(&self.app, user.id, grade).await;
        }
        self.app.session_for(user.id).await
    }

    async fn validate(
        &self,
        token: &str,
        skill: Uuid,
        stage: &str,
    ) -> (StatusCode, serde_json::Value) {
        self.app
            .request(
                Method::PUT,
                &format!(
                    "/api/v1/clubs/{}/members/{}/skill-validations/{}",
                    self.club_slug, self.student_id, skill
                ),
                Some(token),
                Some(json!({ "stage": stage })),
            )
            .await
    }
}

#[tokio::test]
async fn grade_gate_follows_the_level_ladder() {
    // N2 requires E2: a plain member, a stagiaire MF1 and an E1 are refused,
    // an E2 and an E3 pass.
    let ctx = setup("grades", "N2").await;

    let plain = ctx.validator("plain@example.com", None).await;
    let (status, _) = ctx.validate(&plain, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Stagiaire MF1 = dated status only, grants nothing (ADR-0007 Q5).
    let stagiaire_user = ctx
        .app
        .create_user("stagiaire@example.com", true, false)
        .await;
    ctx.app
        .add_membership(ctx.club_id, stagiaire_user.id, ctx.member_role_id)
        .await;
    {
        use crate::users::entities::user_training_statuses;
        use sea_orm::{ActiveModelTrait, Set};
        user_training_statuses::ActiveModel {
            id: Set(Uuid::now_v7()),
            user_id: Set(stagiaire_user.id),
            status: Set("stagiaire_mf1".to_string()),
            valid_until: Set(chrono::NaiveDate::from_ymd_opt(2030, 1, 1).unwrap()),
            created_at: Set(crate::now()),
        }
        .insert(&ctx.app.state.db)
        .await
        .expect("status");
    }
    let stagiaire = ctx.app.session_for(stagiaire_user.id).await;
    let (status, _) = ctx.validate(&stagiaire, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let e1 = ctx.validator("e1@example.com", Some("E1")).await;
    let (status, body) = ctx.validate(&e1, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("certified E2 instructor"));

    let e2 = ctx.validator("e2@example.com", Some("E2")).await;
    let (status, body) = ctx.validate(&e2, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let e3 = ctx.validator("e3@example.com", Some("E3")).await;
    let (status, _) = ctx.validate(&e3, ctx.skill_ids[0], "a_perfectionner").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn e1_validates_n1_but_not_n2() {
    let ctx = setup("ladder", "N1").await;
    let e1 = ctx.validator("e1@example.com", Some("E1")).await;
    let (status, body) = ctx.validate(&e1, ctx.skill_ids[0], "valide").await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn regressions_and_deletions_are_real_global_admin_only() {
    let ctx = setup("rollback", "N2").await;
    let e3 = ctx.validator("e3@example.com", Some("E3")).await;
    let (status, _) = ctx.validate(&e3, ctx.skill_ids[0], "valide").await;
    assert_eq!(status, StatusCode::OK);

    // Re-stamping the SAME stage is allowed (updates date/validator).
    let (status, _) = ctx.validate(&e3, ctx.skill_ids[0], "valide").await;
    assert_eq!(status, StatusCode::OK);

    // Going backwards: 409 for the instructor...
    let (status, _) = ctx.validate(&e3, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::CONFLICT);

    // ...and for a club admin (not a platform admin).
    let admin_role = {
        use crate::tenancy::entities::club_roles;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        club_roles::Entity::find()
            .filter(club_roles::Column::ClubId.eq(ctx.club_id))
            .filter(club_roles::Column::IsAdmin.eq(true))
            .one(&ctx.app.state.db)
            .await
            .expect("query")
            .expect("admin role")
    };
    let club_admin = ctx
        .app
        .create_user("club-admin@example.com", true, false)
        .await;
    ctx.app
        .add_membership(ctx.club_id, club_admin.id, admin_role.id)
        .await;
    support::make_instructor(&ctx.app, club_admin.id, "E4").await;
    let club_admin_token = ctx.app.session_for(club_admin.id).await;
    let (status, _) = ctx
        .validate(&club_admin_token, ctx.skill_ids[0], "vu")
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // DELETE is refused even to an E4 club admin.
    let delete_path = format!(
        "/api/v1/clubs/{}/members/{}/skill-validations/{}",
        ctx.club_slug, ctx.student_id, ctx.skill_ids[0]
    );
    let (status, _) = ctx
        .app
        .request(Method::DELETE, &delete_path, Some(&club_admin_token), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The real global admin regresses (event: reverted) and deletes.
    let (status, body) = ctx.validate(&ctx.root_token, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = ctx
        .app
        .request(Method::DELETE, &delete_path, Some(&ctx.root_token), None)
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, events) = ctx
        .app
        .request(
            Method::GET,
            &format!(
                "/api/v1/competencies/validation-events?user_id={}",
                ctx.student_id
            ),
            Some(&ctx.root_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{events}");
    let actions: Vec<&str> = events["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    // Newest first: deleted, reverted, then the two successful validated
    // writes (the 409 regression attempts produced NO event).
    assert_eq!(
        actions,
        vec!["deleted", "reverted", "validated", "validated"]
    );
}

#[tokio::test]
async fn impersonation_gains_no_competency_writes() {
    let ctx = setup("imper", "N2").await;
    // The impersonated target is an E3 instructor — Fosse would have let the
    // admin validate under their name.
    let e3_user = ctx.app.create_user("e3@example.com", true, false).await;
    ctx.app
        .add_membership(ctx.club_id, e3_user.id, ctx.member_role_id)
        .await;
    support::make_instructor(&ctx.app, e3_user.id, "E3").await;

    let (status, body) = ctx
        .app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/{}/impersonate", ctx.club_slug),
            Some(&ctx.root_token),
            Some(json!({ "user_id": e3_user.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let imp_token = body["token"].as_str().unwrap().to_string();

    // Reads stay allowed ("view as")...
    let (status, _) = ctx
        .app
        .request(
            Method::GET,
            &format!(
                "/api/v1/clubs/{}/members/{}/competencies?certification=N2",
                ctx.club_slug, ctx.student_id
            ),
            Some(&imp_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // ...writes are not.
    let (status, _) = ctx.validate(&imp_token, ctx.skill_ids[0], "vu").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn self_validation_is_refused() {
    let ctx = setup("selfv", "N2").await;
    let e3_user = ctx.app.create_user("e3@example.com", true, false).await;
    ctx.app
        .add_membership(ctx.club_id, e3_user.id, ctx.member_role_id)
        .await;
    support::make_instructor(&ctx.app, e3_user.id, "E3").await;
    support::set_preparing(&ctx.app, e3_user.id, "N2").await;
    let token = ctx.app.session_for(e3_user.id).await;

    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            &format!(
                "/api/v1/clubs/{}/members/{}/skill-validations/{}",
                ctx.club_slug, e3_user.id, ctx.skill_ids[0]
            ),
            Some(&token),
            Some(json!({ "stage": "vu" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn instructors_reach_the_students_list_plain_members_do_not() {
    let ctx = setup("students", "N2").await;
    let path = format!(
        "/api/v1/clubs/{}/competencies/students?certification=N2",
        ctx.club_slug
    );

    let plain = ctx.validator("plain@example.com", None).await;
    let (status, _) = ctx
        .app
        .request(Method::GET, &path, Some(&plain), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let e2 = ctx.validator("e2@example.com", Some("E2")).await;
    let (status, body) = ctx.app.request(Method::GET, &path, Some(&e2), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["user"]["id"], ctx.student_id.to_string());
    assert_eq!(body["items"][0]["progress"]["total"], 2);

    // Pagination regression (review HIGH-1): explicit page params must parse
    // — with #[serde(flatten)] this was a 400.
    let (status, body) = ctx
        .app
        .request(
            Method::GET,
            &format!("{path}&page=2&per_page=1"),
            Some(&e2),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["page"], 2);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn global_admin_attributes_validations_to_an_instructor() {
    // Product decision 2026-08-20: a real global admin may validate "au nom
    // d'un encadrant" — the designated instructor must be a legitimate
    // validator (active co-member, sufficient derived grade).
    let ctx = setup("behalf", "N2").await;
    let e3_user = ctx.app.create_user("e3@example.com", true, false).await;
    ctx.app
        .add_membership(ctx.club_id, e3_user.id, ctx.member_role_id)
        .await;
    support::make_instructor(&ctx.app, e3_user.id, "E3").await;

    // An instructor cannot attribute to someone else.
    let e2_token = ctx.validator("e2@example.com", Some("E2")).await;
    let path = format!(
        "/api/v1/clubs/{}/members/{}/skill-validations/{}",
        ctx.club_slug, ctx.student_id, ctx.skill_ids[0]
    );
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            &path,
            Some(&e2_token),
            Some(json!({ "stage": "vu", "validated_by_user_id": e3_user.id })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Designated validator must hold the grade…
    let plain = ctx.app.create_user("plain@example.com", true, false).await;
    ctx.app
        .add_membership(ctx.club_id, plain.id, ctx.member_role_id)
        .await;
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            &path,
            Some(&ctx.root_token),
            Some(json!({ "stage": "vu", "validated_by_user_id": plain.id })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // …be a member of THIS club…
    let outsider = ctx.app.create_user("out@example.com", true, false).await;
    support::make_instructor(&ctx.app, outsider.id, "E3").await;
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            &path,
            Some(&ctx.root_token),
            Some(json!({ "stage": "vu", "validated_by_user_id": outsider.id })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // …and not be the diver (rule 7 targets the RECORDED validator).
    let (status, _) = ctx
        .app
        .request(
            Method::PUT,
            &path,
            Some(&ctx.root_token),
            Some(json!({ "stage": "vu", "validated_by_user_id": ctx.student_id })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Valid attribution: the recorded validator is the instructor.
    let (status, view) = ctx
        .app
        .request(
            Method::PUT,
            &path,
            Some(&ctx.root_token),
            Some(json!({ "stage": "valide", "validated_by_user_id": e3_user.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let validated_by = &view["domains"][0]["modules"][0]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == ctx.skill_ids[0].to_string())
        .unwrap()["validation"]["validated_by"];
    assert_eq!(validated_by["id"], e3_user.id.to_string());
}
