use axum::http::{Method, StatusCode};
use sea_orm::*;
use serde_json::json;

use crate::identity::entities::{prelude::Users, users};
use crate::test_support::TestApp;
use crate::users::entities::{prelude::UserProfiles, user_profiles};

struct Club {
    slug: &'static str,
    id: uuid::Uuid,
    admin_token: String,
    admin_role: uuid::Uuid,
    member_role: uuid::Uuid,
}

async fn club_with_admin(app: &TestApp, slug: &'static str) -> Club {
    let (club, admin_role, member_role) = app.create_club(slug).await;
    let admin = app
        .create_user(&format!("admin@{slug}.test"), true, false)
        .await;
    app.add_membership(club.id, admin.id, admin_role.id).await;
    Club {
        slug,
        id: club.id,
        admin_token: app.session_for(admin.id).await,
        admin_role: admin_role.id,
        member_role: member_role.id,
    }
}

async fn add(
    app: &TestApp,
    club: &Club,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    app.request(
        Method::POST,
        &format!("/api/v1/clubs/{}/members", club.slug),
        Some(token),
        Some(body),
    )
    .await
}

async fn profile(app: &TestApp, user_id: uuid::Uuid) -> Option<user_profiles::Model> {
    UserProfiles::find()
        .filter(user_profiles::Column::UserId.eq(user_id))
        .one(&app.state.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn admin_creates_member_without_email() {
    let app = TestApp::new().await;
    let club = club_with_admin(&app, "usi").await;
    let (status, body) = add(
        &app,
        &club,
        &club.admin_token,
        json!({ "first_name": " Lucas ", "last_name": "Martin",
                "birthdate": "2012-05-03", "role_id": club.member_role }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["outcome"], "created");
    assert!(body["user"]["email"].is_null());

    let user_id: uuid::Uuid = body["user"]["id"].as_str().unwrap().parse().unwrap();
    let user = Users::find_by_id(user_id)
        .one(&app.state.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.first_name, "Lucas");
    assert!(user.activated_at.is_none(), "never logged in");
    assert_eq!(
        profile(&app, user_id)
            .await
            .unwrap()
            .birthdate
            .unwrap()
            .to_string(),
        "2012-05-03"
    );
    assert!(
        app.state.mailer.captured().is_empty(),
        "no address, no welcome email"
    );
}

#[tokio::test]
async fn admin_creates_member_with_email_and_welcome_mail() {
    let app = TestApp::new().await;
    let club = club_with_admin(&app, "usi").await;
    let (status, body) = add(
        &app,
        &club,
        &club.admin_token,
        json!({ "first_name": "Marie", "last_name": "Leroy",
                "email": " Marie.Leroy@Example.com ", "role_id": club.member_role }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["email"], "marie.leroy@example.com");

    let mails = app.state.mailer.captured();
    let welcome = mails.last().expect("welcome email");
    assert_eq!(welcome.to, "marie.leroy@example.com");
    assert!(welcome
        .body
        .contains("/login?email=marie.leroy%40example.com"));
    assert!(
        !welcome.body.contains("/auth/"),
        "no token in a welcome email"
    );

    // The new member can log in by magic link straight away.
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link",
            None,
            Some(json!({ "email": "marie.leroy@example.com" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let token = app.last_email_token("/auth/ml/");
    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/magic-link/consume",
            None,
            Some(json!({ "token": token })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn existing_account_joins_without_being_overwritten() {
    let app = TestApp::new().await;
    let club_a = club_with_admin(&app, "club-a").await;
    let club_b = club_with_admin(&app, "club-b").await;

    // Known in club B, with a birthdate but no phone.
    let (status, created) = add(
        &app,
        &club_b,
        &club_b.admin_token,
        json!({ "first_name": "Paul", "last_name": "Durand", "email": "paul@example.com",
                "birthdate": "1985-03-12", "role_id": club_b.member_role, "send_welcome": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let paul_id: uuid::Uuid = created["user"]["id"].as_str().unwrap().parse().unwrap();

    // Club A enters the same email with other values.
    let (status, body) = add(
        &app,
        &club_a,
        &club_a.admin_token,
        json!({ "first_name": "Paolo", "last_name": "Durand", "email": "PAUL@example.com",
                "birthdate": "1990-01-01", "phone": "0601020304",
                "role_id": club_a.member_role, "send_welcome": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["outcome"], "added_existing");
    assert_eq!(
        body["user"]["id"], created["user"]["id"],
        "same person, no duplicate"
    );
    let fields: Vec<&str> = body["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["name", "birthdate"]);

    let user = Users::find_by_id(paul_id)
        .one(&app.state.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.first_name, "Paul", "name never overwritten");
    let stored = profile(&app, paul_id).await.unwrap();
    assert_eq!(stored.birthdate.unwrap().to_string(), "1985-03-12", "kept");
    assert_eq!(
        stored.phone.as_deref(),
        Some("0601020304"),
        "empty field filled"
    );

    // Adding them again is a conflict.
    let (status, _) = add(
        &app,
        &club_a,
        &club_a.admin_token,
        json!({ "first_name": "Paul", "last_name": "Durand", "email": "paul@example.com",
                "role_id": club_a.member_role }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn intake_requires_permission_and_stays_in_the_club() {
    let app = TestApp::new().await;
    let club_a = club_with_admin(&app, "club-a").await;
    let club_b = club_with_admin(&app, "club-b").await;
    let body =
        |role: uuid::Uuid| json!({ "first_name": "Eve", "last_name": "Intruder", "role_id": role });

    // Plain member: no intake permission.
    let member = app.create_user("member@club-a.test", true, false).await;
    app.add_membership(club_a.id, member.id, club_a.member_role)
        .await;
    let member_token = app.session_for(member.id).await;
    let (status, _) = add(&app, &club_a, &member_token, body(club_a.member_role)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Club B's admin cannot add into club A.
    let (status, _) = add(&app, &club_a, &club_b.admin_token, body(club_a.member_role)).await;
    assert!(
        status == StatusCode::FORBIDDEN || status == StatusCode::NOT_FOUND,
        "foreign admin refused, got {status}"
    );

    // A role of another club is invisible.
    let (status, _) = add(&app, &club_a, &club_a.admin_token, body(club_b.member_role)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Intake-only staff cannot grant the admin role.
    let staff_role = app
        .create_role(club_a.id, "Secrétariat", &["club.intake.manage"])
        .await;
    let staff = app.create_user("staff@club-a.test", true, false).await;
    app.add_membership(club_a.id, staff.id, staff_role.id).await;
    let staff_token = app.session_for(staff.id).await;
    let (status, _) = add(&app, &club_a, &staff_token, body(club_a.admin_role)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = add(&app, &club_a, &staff_token, body(club_a.member_role)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "intake permission is enough for a member"
    );

    let intruders = Users::find()
        .filter(users::Column::LastName.eq("Intruder"))
        .count(&app.state.db)
        .await
        .unwrap();
    assert_eq!(intruders, 1, "only the authorized add created an account");
}

#[tokio::test]
async fn intake_is_refused_while_impersonating() {
    let app = TestApp::new().await;
    let club = club_with_admin(&app, "usi").await;
    let target = app.create_user("target@usi.test", true, false).await;
    app.add_membership(club.id, target.id, club.member_role)
        .await;
    let (status, session) = app
        .request(
            Method::POST,
            &format!("/api/v1/clubs/{}/impersonate", club.slug),
            Some(&club.admin_token),
            Some(json!({ "user_id": target.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let token = session["token"].as_str().unwrap();
    let (status, _) = add(
        &app,
        &club,
        token,
        json!({ "first_name": "A", "last_name": "B", "role_id": club.member_role }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn platform_admin_adds_a_club_administrator() {
    let app = TestApp::new().await;
    let (club, _, _) = app.create_club("usi").await;
    let root = app.create_user("root@example.com", true, true).await;
    let root_token = app.session_for(root.id).await;
    let path = format!("/api/v1/platform/clubs/{}/admins", club.id);
    let body = json!({ "email": "boss@usi.test", "first_name": "Boss", "last_name": "Usi" });

    // A club admin (not global) cannot.
    let other = app.create_user("other@example.com", true, false).await;
    let other_token = app.session_for(other.id).await;
    let (status, _) = app
        .request(Method::POST, &path, Some(&other_token), Some(body.clone()))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, created) = app
        .request(Method::POST, &path, Some(&root_token), Some(body))
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["role_name"], "Administrateur");
    assert_eq!(
        app.state.mailer.captured().last().unwrap().to,
        "boss@usi.test"
    );

    // Email is mandatory for a club administrator.
    let (status, _) = app
        .request(
            Method::POST,
            &path,
            Some(&root_token),
            Some(json!({ "email": "", "first_name": "No", "last_name": "Mail" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
