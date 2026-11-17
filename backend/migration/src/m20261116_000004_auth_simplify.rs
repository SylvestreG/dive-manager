//! Auth simplification — slice 1 of the auth & onboarding redesign
//! (docs/design/auth-onboarding.md, ADR-0009):
//! - passwords and invitations are removed (tables dropped);
//! - `users.email` becomes optional (members imported without an address);
//! - the `invited` status is replaced by `activated_at` (first login) and
//!   `email_verified_at` (mailbox proven by a magic link);
//! - permission `club.invitations.manage` is renamed `club.intake.manage`.
//!
//! Forward-only: dropped credentials and invitations cannot be restored.
//!
//! Justified raw DDL (ADR-0006 §1): sea-query cannot drop or add a named
//! CHECK constraint on an existing table, needed to retire `invited` from
//! `users_status_check` (the name PostgreSQL gave the column CHECK created by
//! the foundation migration).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. New lifecycle columns + optional email.
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::Email).text().null())
                    .add_column(
                        ColumnDef::new(Users::ActivatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Users::EmailVerifiedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Backfill: every non-`invited` user has logged in (invitation
        //    acceptance and magic-link consumption both opened a session) —
        //    including `disabled` ones, which must NOT come back as "never
        //    logged in" (that would make them club-managed, design R4).
        //    Both proofs went through their mailbox.
        manager
            .exec_stmt(
                Query::update()
                    .table(Users::Table)
                    .value(Users::ActivatedAt, Expr::col(Users::CreatedAt))
                    .value(Users::EmailVerifiedAt, Expr::col(Users::CreatedAt))
                    .and_where(Expr::col(Users::Status).ne("invited"))
                    .to_owned(),
            )
            .await?;
        manager
            .exec_stmt(
                Query::update()
                    .table(Users::Table)
                    .value(Users::Status, "active")
                    .and_where(Expr::col(Users::Status).eq("invited"))
                    .to_owned(),
            )
            .await?;

        // 3. Retire `invited` from the status CHECK (raw DDL, see header).
        let db = manager.get_connection();
        db.execute_unprepared("ALTER TABLE users DROP CONSTRAINT IF EXISTS users_status_check")
            .await?;
        db.execute_unprepared(
            "ALTER TABLE users ADD CONSTRAINT users_status_check \
             CHECK (status IN ('active', 'disabled'))",
        )
        .await?;

        // 4. Permission rename (grants are data, the catalogue is code).
        manager
            .exec_stmt(
                Query::update()
                    .table(ClubRolePermissions::Table)
                    .value(ClubRolePermissions::Permission, "club.intake.manage")
                    .and_where(
                        Expr::col(ClubRolePermissions::Permission).eq("club.invitations.manage"),
                    )
                    .to_owned(),
            )
            .await?;

        // 5. Drop removed features (the invitations partial index goes with
        //    its table).
        for table in [
            UserPasswords::Table.into_iden(),
            Invitations::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "m20261116_000004_auth_simplify is forward-only (credentials and \
             invitations were dropped)"
                .to_string(),
        ))
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Email,
    Status,
    ActivatedAt,
    EmailVerifiedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ClubRolePermissions {
    Table,
    Permission,
}

#[derive(DeriveIden)]
enum UserPasswords {
    Table,
}

#[derive(DeriveIden)]
enum Invitations {
    Table,
}
