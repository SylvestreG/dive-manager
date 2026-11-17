use std::time::Duration as StdDuration;

use chrono::Duration;
use sea_orm::*;

use crate::identity::entities::{auth_sessions, login_magic_links, prelude::*};
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
