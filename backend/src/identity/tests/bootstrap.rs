use sea_orm::*;

use crate::bootstrap;
use crate::identity::entities::{prelude::*, users};
use crate::test_support::TestApp;

#[tokio::test]
async fn bootstrap_is_config_seeded_then_database_owned() {
    let app = TestApp::new().await;
    let mut config = (*app.state.config).clone();
    config.auth.bootstrap_admin_emails = vec!["First@Example.com".to_string()];
    bootstrap::bootstrap_global_admins(&app.state.db, &config)
        .await
        .expect("bootstrap");
    bootstrap::bootstrap_global_admins(&app.state.db, &config)
        .await
        .expect("bootstrap twice");

    let admins = Users::find()
        .filter(users::Column::IsGlobalAdmin.eq(true))
        .all(&app.state.db)
        .await
        .unwrap();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0].email, "first@example.com");
    assert_eq!(admins[0].status, users::STATUS_INVITED);

    // Once an admin exists, the config list is ignored entirely.
    config.auth.bootstrap_admin_emails = vec!["second@example.com".to_string()];
    bootstrap::bootstrap_global_admins(&app.state.db, &config)
        .await
        .expect("bootstrap ignored");
    let count = Users::find()
        .filter(users::Column::IsGlobalAdmin.eq(true))
        .count(&app.state.db)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
