//! Shared test harness (compiled only for tests).
//!
//! Requires `TEST_DATABASE_URL` pointing to a PostgreSQL *server* (any
//! maintenance database); each TestApp creates its own throwaway database so
//! tests run in parallel with full isolation.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use migration::{Migrator, MigratorTrait};
use sea_orm::*;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use crate::config::Config;
use crate::identity::entities::users;
use crate::identity::services::sessions;
use crate::mailer::Mailer;
use crate::rate_limit::RateLimiter;
use crate::tenancy::entities::{club_role_permissions, club_roles, clubs, memberships, prelude::*};
use crate::tenancy::services::seed;
use crate::{app, normalize_email, now, AppState};

/// Creates a throwaway, NOT migrated database; returns it with its URL.
/// Migration tests use it to stop at an earlier schema version.
pub async fn empty_database() -> (DatabaseConnection, String) {
    let base_url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must point to a PostgreSQL server \
         (e.g. postgres://postgres@localhost:5499/postgres)",
    );
    let db_name = format!("dm_test_{}", Uuid::now_v7().simple());
    let admin_db = Database::connect(&base_url)
        .await
        .expect("connect admin db");
    // Test infrastructure only: creating throwaway databases is DDL that
    // no application code path uses (ADR-0006 targets runtime queries).
    admin_db
        .execute_unprepared(&format!("CREATE DATABASE {db_name}"))
        .await
        .expect("create test database");
    let (server, _) = base_url.rsplit_once('/').expect("url with database path");
    let db_url = format!("{server}/{db_name}");
    let db = Database::connect(&db_url).await.expect("connect test db");
    (db, db_url)
}

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
}

impl TestApp {
    pub async fn new() -> TestApp {
        TestApp::with_config(|_| {}).await
    }

    /// Like `new`, with a hook to adjust the configuration (e.g. point OIDC
    /// providers at a local fake).
    pub async fn with_config(adjust: impl FnOnce(&mut Config)) -> TestApp {
        let (db, db_url) = empty_database().await;
        Migrator::up(&db, None).await.expect("migrations");

        let mut config = Config::default();
        config.database.url = db_url;
        config.base_url = "http://test".to_string();
        adjust(&mut config);
        let state = AppState {
            db,
            config: Arc::new(config),
            mailer: Arc::new(Mailer::capture()),
            rate: Arc::new(RateLimiter::default()),
            oidc: Default::default(),
        };
        TestApp {
            router: app::build_router(state.clone()),
            state,
        }
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let (status, value, _) = self
            .request_with_cookie(method, path, token, body, None)
            .await;
        (status, value)
    }

    /// JSON request carrying an optional `Cookie` header; also returns the
    /// `Set-Cookie` header of the response.
    pub async fn request_with_cookie(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
        cookie: Option<&str>,
    ) -> (StatusCode, Value, Option<String>) {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header("Authorization", format!("Bearer {token}"));
        }
        if let Some(cookie) = cookie {
            builder = builder.header("Cookie", cookie);
        }
        let request = match body {
            Some(body) => builder
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        }
        .expect("request");
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("response");
        let status = response.status();
        let set_cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, value, set_cookie)
    }

    /// `activated`: whether the account has already logged in once
    /// (`activated_at` set) — ADR-0009 replaced the `invited` status.
    /// A browser navigation: returns the status and the `Location` header
    /// (redirect endpoints such as the OIDC start/callback).
    pub async fn navigate(
        &self,
        method: Method,
        path: &str,
        form: Option<&str>,
    ) -> (StatusCode, String) {
        let (status, location, _) = self.navigate_with_cookie(method, path, form, None).await;
        (status, location)
    }

    /// Navigation with an optional `Cookie` header; returns the status, the
    /// `Location` and the `Set-Cookie` headers.
    pub async fn navigate_with_cookie(
        &self,
        method: Method,
        path: &str,
        form: Option<&str>,
        cookie: Option<&str>,
    ) -> (StatusCode, String, Option<String>) {
        let mut builder = Request::builder().method(method).uri(path).header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Chrome/130.0 Safari/537.36",
        );
        if let Some(cookie) = cookie {
            builder = builder.header("Cookie", cookie);
        }
        let request = match form {
            Some(body) => builder
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        }
        .expect("request");
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("response");
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let set_cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        (response.status(), location, set_cookie)
    }

    pub async fn create_user(
        &self,
        email: &str,
        activated: bool,
        global_admin: bool,
    ) -> users::Model {
        let created = now();
        users::ActiveModel {
            id: Set(Uuid::now_v7()),
            email: Set(Some(normalize_email(email))),
            first_name: Set("Test".to_string()),
            last_name: Set("User".to_string()),
            status: Set(users::STATUS_ACTIVE.to_string()),
            is_global_admin: Set(global_admin),
            activated_at: Set(activated.then_some(created)),
            email_verified_at: Set(activated.then_some(created)),
            created_at: Set(created),
            updated_at: Set(created),
        }
        .insert(&self.state.db)
        .await
        .expect("create user")
    }

    pub async fn session_for(&self, user_id: Uuid) -> String {
        let (token, _) =
            sessions::create_session(&self.state.db, &self.state.config, user_id, None, None)
                .await
                .expect("session");
        token
    }

    /// Creates a club with the two seeded roles (ADR-0007);
    /// returns (club, admin_role, member_role).
    pub async fn create_club(
        &self,
        slug: &str,
    ) -> (clubs::Model, club_roles::Model, club_roles::Model) {
        let created = now();
        let club = clubs::ActiveModel {
            id: Set(Uuid::now_v7()),
            name: Set(format!("Club {slug}")),
            slug: Set(slug.to_string()),
            status: Set(clubs::STATUS_ACTIVE.to_string()),
            created_at: Set(created),
            updated_at: Set(created),
        }
        .insert(&self.state.db)
        .await
        .expect("club");
        seed::seed_club(&self.state.db, club.id)
            .await
            .expect("seed");
        let roles = ClubRoles::find()
            .filter(club_roles::Column::ClubId.eq(club.id))
            .all(&self.state.db)
            .await
            .expect("roles");
        let find = |name: &str| {
            roles
                .iter()
                .find(|r| r.name == name)
                .cloned()
                .expect("seeded role")
        };
        (club, find(seed::ROLE_ADMIN), find(seed::ROLE_MEMBER))
    }

    /// Creates an extra club role with explicit permission grants (ADR-0007:
    /// the seed has no staff role — tests build their own personas).
    pub async fn create_role(
        &self,
        club_id: Uuid,
        name: &str,
        permissions: &[&str],
    ) -> club_roles::Model {
        let role = club_roles::ActiveModel {
            id: Set(Uuid::now_v7()),
            club_id: Set(club_id),
            name: Set(name.to_string()),
            is_admin: Set(false),
        }
        .insert(&self.state.db)
        .await
        .expect("create role");
        for permission in permissions {
            club_role_permissions::ActiveModel {
                role_id: Set(role.id),
                permission: Set(String::from(*permission)),
            }
            .insert(&self.state.db)
            .await
            .expect("grant permission");
        }
        role
    }

    pub async fn add_membership(&self, club_id: Uuid, user_id: Uuid, role_id: Uuid) -> Uuid {
        let created = now();
        let membership = memberships::ActiveModel {
            id: Set(Uuid::now_v7()),
            club_id: Set(club_id),
            user_id: Set(user_id),
            role_id: Set(role_id),
            status: Set(memberships::STATUS_ACTIVE.to_string()),
            created_at: Set(created),
            updated_at: Set(created),
        }
        .insert(&self.state.db)
        .await
        .expect("membership");
        membership.id
    }

    /// Extracts the token embedded in the most recent captured email whose
    /// body contains `path_marker` (e.g. "/auth/ml/").
    pub fn last_email_token(&self, path_marker: &str) -> String {
        let mails = self.state.mailer.captured();
        let body = &mails
            .iter()
            .rev()
            .find(|m| m.body.contains(path_marker))
            .expect("captured email with marker")
            .body;
        let start = body.find(path_marker).unwrap() + path_marker.len();
        body[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }
}
