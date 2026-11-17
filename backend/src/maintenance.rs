use std::time::Duration as StdDuration;

use chrono::Duration;
use sea_orm::*;

use crate::identity::entities::{
    auth_sessions, exchange_codes, login_magic_links, oidc_flows, prelude::*,
};
use crate::now;

/// Hourly in-process cleanup of expired secrets — no external job runner
/// (charter: no infrastructure without a demonstrated need).
pub fn spawn_cleanup_task(db: DatabaseConnection) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(StdDuration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = cleanup_once(&db).await {
                tracing::error!(error = ?e, "maintenance cleanup failed");
            }
        }
    });
}

async fn cleanup_once(db: &DatabaseConnection) -> Result<(), DbErr> {
    let current = now();

    // OIDC round-trips (10 min) and exchange codes (60 s): keep a day for
    // diagnostics, then drop.
    OidcFlows::delete_many()
        .filter(oidc_flows::Column::ExpiresAt.lt(current - Duration::days(1)))
        .exec(db)
        .await?;
    ExchangeCodes::delete_many()
        .filter(exchange_codes::Column::ExpiresAt.lt(current - Duration::days(1)))
        .exec(db)
        .await?;

    LoginMagicLinks::delete_many()
        .filter(login_magic_links::Column::ExpiresAt.lt(current - Duration::days(7)))
        .exec(db)
        .await?;

    AuthSessions::delete_many()
        .filter(
            Condition::any()
                .add(auth_sessions::Column::ExpiresAt.lt(current - Duration::days(30)))
                .add(auth_sessions::Column::RevokedAt.lt(current - Duration::days(30))),
        )
        .exec(db)
        .await?;

    Ok(())
}
