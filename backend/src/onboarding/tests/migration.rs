//! `m20261116_000004_auth_simplify` on data written with the previous schema.
//! Rows are inserted through sea-query with string identifiers because the
//! current entities no longer match the old schema.

use migration::{Migrator, MigratorTrait};
use sea_orm::sea_query::{Alias, Expr, PostgresQueryBuilder, Query};
use sea_orm::*;
use uuid::Uuid;

use crate::identity::entities::{prelude::Users, users};
use crate::now;
use crate::tenancy::entities::{club_role_permissions, prelude::ClubRolePermissions};
use crate::test_support::empty_database;

async fn insert(db: &DatabaseConnection, table: &str, columns: &[&str], values: Vec<Value>) {
    let mut stmt = Query::insert();
    stmt.into_table(Alias::new(table))
        .columns(columns.iter().map(|c| Alias::new(*c)))
        .values_panic(values.into_iter().map(|v| Expr::val(v).into()));
    let (sql, params) = stmt.build(PostgresQueryBuilder);
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        params,
    ))
    .await
    .unwrap_or_else(|e| panic!("insert into {table}: {e}"));
}

#[tokio::test]
async fn migrates_invited_users_passwords_invitations_and_grants() {
    let (db, _) = empty_database().await;
    Migrator::up(&db, Some(3)).await.expect("migrate to v3");

    let at = now();
    let invited = Uuid::now_v7();
    let active = Uuid::now_v7();
    let disabled = Uuid::now_v7();
    let user_cols = [
        "id",
        "email",
        "first_name",
        "last_name",
        "status",
        "is_global_admin",
        "created_at",
        "updated_at",
    ];
    for (id, email, status) in [
        (invited, "invited@example.com", "invited"),
        (active, "active@example.com", "active"),
        (disabled, "disabled@example.com", "disabled"),
    ] {
        insert(
            &db,
            "users",
            &user_cols,
            vec![
                id.into(),
                email.into(),
                "First".into(),
                "Last".into(),
                status.into(),
                false.into(),
                at.into(),
                at.into(),
            ],
        )
        .await;
    }
    insert(
        &db,
        "user_passwords",
        &["user_id", "password_hash", "updated_at"],
        vec![active.into(), "$argon2id$dummy".into(), at.into()],
    )
    .await;

    let club = Uuid::now_v7();
    insert(
        &db,
        "clubs",
        &["id", "name", "slug", "status", "created_at", "updated_at"],
        vec![
            club.into(),
            "Club".into(),
            "club".into(),
            "active".into(),
            at.into(),
            at.into(),
        ],
    )
    .await;
    let role = Uuid::now_v7();
    insert(
        &db,
        "club_roles",
        &["id", "club_id", "name", "is_admin"],
        vec![role.into(), club.into(), "Secrétariat".into(), false.into()],
    )
    .await;
    for permission in ["club.invitations.manage", "club.members.view"] {
        insert(
            &db,
            "club_role_permissions",
            &["role_id", "permission"],
            vec![role.into(), permission.into()],
        )
        .await;
    }
    insert(
        &db,
        "invitations",
        &[
            "id",
            "email",
            "first_name",
            "last_name",
            "club_id",
            "role_id",
            "invited_by",
            "token_hash",
            "expires_at",
            "created_at",
        ],
        vec![
            Uuid::now_v7().into(),
            "new@example.com".into(),
            "New".into(),
            "Person".into(),
            club.into(),
            role.into(),
            active.into(),
            vec![1u8; 32].into(),
            at.into(),
            at.into(),
        ],
    )
    .await;

    Migrator::up(&db, None).await.expect("migrate to latest");

    let invited_user = Users::find_by_id(invited).one(&db).await.unwrap().unwrap();
    assert_eq!(invited_user.status, users::STATUS_ACTIVE);
    assert!(
        invited_user.activated_at.is_none(),
        "invited = never logged in"
    );
    let active_user = Users::find_by_id(active).one(&db).await.unwrap().unwrap();
    assert!(active_user.activated_at.is_some());
    assert!(active_user.email_verified_at.is_some());
    let disabled_user = Users::find_by_id(disabled).one(&db).await.unwrap().unwrap();
    assert_eq!(disabled_user.status, users::STATUS_DISABLED);
    assert!(
        disabled_user.activated_at.is_some(),
        "a disabled account had logged in: never club-managed again"
    );

    let grants: Vec<String> = ClubRolePermissions::find()
        .filter(club_role_permissions::Column::RoleId.eq(role))
        .all(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.permission)
        .collect();
    assert!(grants.contains(&"club.intake.manage".to_string()));
    assert!(!grants.contains(&"club.invitations.manage".to_string()));

    for table in ["user_passwords", "invitations"] {
        assert!(
            !migration::SchemaManager::new(&db)
                .has_table(table)
                .await
                .unwrap(),
            "{table} dropped"
        );
    }

    // `invited` is no longer a valid status; NULL email is now accepted.
    let mut invalid: users::ActiveModel = invited_user.clone().into();
    invalid.status = Set("invited".to_string());
    assert!(invalid.update(&db).await.is_err());
    let mut no_email: users::ActiveModel = invited_user.into();
    no_email.email = Set(None);
    no_email.update(&db).await.expect("email is optional");
}
