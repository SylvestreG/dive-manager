use axum::http::{Method, StatusCode};
use chrono::Duration;
use sea_orm::*;

use crate::identity::entities::{auth_sessions, prelude::*};
use crate::now;
use crate::test_support::TestApp;

#[tokio::test]
async fn logout_all_revokes_immediately() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", true, false).await;
    let token_a = app.session_for(user.id).await;
    let token_b = app.session_for(user.id).await;

    let (status, _) = app
        .request(
            Method::POST,
            "/api/v1/auth/logout-all",
            Some(&token_a),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    for token in [&token_a, &token_b] {
        let (status, _) = app
            .request(Method::GET, "/api/v1/me", Some(token), None)
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}

/// Expiry logic tested by rewriting the stored timestamps (no clock
/// abstraction needed): sliding idle expiry and the absolute cap both deny.
#[tokio::test]
async fn expired_sessions_are_rejected() {
    let app = TestApp::new().await;
    let user = app.create_user("diver@example.com", true, false).await;

    // Idle expiry passed.
    let token = app.session_for(user.id).await;
    AuthSessions::update_many()
        .col_expr(
            auth_sessions::Column::ExpiresAt,
            sea_orm::sea_query::Expr::value(now() - Duration::minutes(1)),
        )
        .filter(auth_sessions::Column::UserId.eq(user.id))
        .exec(&app.state.db)
        .await
        .unwrap();
    let (status, _) = app
        .request(Method::GET, "/api/v1/me", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "idle expiry");

    // Absolute cap passed even though the sliding expiry looks fine.
    let token = app.session_for(user.id).await;
    let too_old = now() - Duration::days(app.state.config.auth.session_max_days + 1);
    AuthSessions::update_many()
        .col_expr(
            auth_sessions::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(too_old),
        )
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .filter(auth_sessions::Column::UserId.eq(user.id))
        .exec(&app.state.db)
        .await
        .unwrap();
    let (status, _) = app
        .request(Method::GET, "/api/v1/me", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "absolute cap");
}
