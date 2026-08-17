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
use crate::tenancy::entities::{club_roles, clubs, memberships, prelude::*};
use crate::tenancy::services::seed;
use crate::{app, normalize_email, now, AppState};

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
}

impl TestApp {
    pub async fn new() -> TestApp {
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
        Migrator::up(&db, None).await.expect("migrations");

        let mut config = Config::default();
        config.database.url = db_url;
        config.base_url = "http://test".to_string();
        let state = AppState {
            db,
            config: Arc::new(config),
            mailer: Arc::new(Mailer::capture()),
            rate: Arc::new(RateLimiter::default()),
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
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header("Authorization", format!("Bearer {token}"));
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
        (status, value)
    }

    pub async fn create_user(&self, email: &str, active: bool, global_admin: bool) -> users::Model {
        let created = now();
        users::ActiveModel {
            id: Set(Uuid::now_v7()),
            email: Set(normalize_email(email)),
            first_name: Set("Test".to_string()),
            last_name: Set("User".to_string()),
            status: Set(if active {
                users::STATUS_ACTIVE.to_string()
            } else {
                users::STATUS_INVITED.to_string()
            }),
            is_global_admin: Set(global_admin),
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

    /// Creates a club with seeded roles;
    /// returns (club, admin_role, instructor_role, member_role).
    pub async fn create_club(
        &self,
        slug: &str,
    ) -> (
        clubs::Model,
        club_roles::Model,
        club_roles::Model,
        club_roles::Model,
    ) {
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
        (
            club,
            find(seed::ROLE_ADMIN),
            find(seed::ROLE_INSTRUCTOR),
            find(seed::ROLE_MEMBER),
        )
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
