//! Foundation schema: identity (users, credentials, sessions, invitations,
//! audit) and tenancy (clubs, roles, memberships, module toggles).
//!
//! Conventions (ADR-0006): SeaORM DSL only, no raw SQL. Emails and slugs are
//! stored lowercase, enforced by CHECK constraints instead of citext.
//! One justified exception (ADR-0006 §1): the partial unique index
//! `uq_invitations_pending` — sea-query cannot express partial indexes.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn timestamps(table: &mut TableCreateStatement, created: impl IntoIden, updated: impl IntoIden) {
    table
        .col(
            ColumnDef::new(created)
                .timestamp_with_time_zone()
                .not_null(),
        )
        .col(
            ColumnDef::new(updated)
                .timestamp_with_time_zone()
                .not_null(),
        );
}

fn lowercase_check(col: impl IntoIden) -> SimpleExpr {
    let col = col.into_iden();
    Expr::col(col.clone()).eq(Func::lower(Expr::col(col)))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ---------- users ----------
        let mut users = Table::create();
        users
            .table(Users::Table)
            .col(ColumnDef::new(Users::Id).uuid().primary_key())
            .col(
                ColumnDef::new(Users::Email)
                    .text()
                    .not_null()
                    .unique_key()
                    .check(lowercase_check(Users::Email)),
            )
            .col(
                ColumnDef::new(Users::FirstName)
                    .text()
                    .not_null()
                    .check(Expr::col(Users::FirstName).ne("")),
            )
            .col(ColumnDef::new(Users::LastName).text().not_null())
            .col(
                ColumnDef::new(Users::Status)
                    .text()
                    .not_null()
                    .check(Expr::col(Users::Status).is_in(["invited", "active", "disabled"])),
            )
            .col(
                ColumnDef::new(Users::IsGlobalAdmin)
                    .boolean()
                    .not_null()
                    .default(false),
            );
        timestamps(&mut users, Users::CreatedAt, Users::UpdatedAt);
        manager.create_table(users).await?;

        // ---------- clubs ----------
        let mut clubs = Table::create();
        clubs
            .table(Clubs::Table)
            .col(ColumnDef::new(Clubs::Id).uuid().primary_key())
            .col(ColumnDef::new(Clubs::Name).text().not_null())
            .col(
                ColumnDef::new(Clubs::Slug)
                    .text()
                    .not_null()
                    .unique_key()
                    .check(lowercase_check(Clubs::Slug)),
            )
            .col(
                ColumnDef::new(Clubs::Status)
                    .text()
                    .not_null()
                    .check(Expr::col(Clubs::Status).is_in(["active", "suspended"])),
            );
        timestamps(&mut clubs, Clubs::CreatedAt, Clubs::UpdatedAt);
        manager.create_table(clubs).await?;

        // ---------- club_roles ----------
        manager
            .create_table(
                Table::create()
                    .table(ClubRoles::Table)
                    .col(ColumnDef::new(ClubRoles::Id).uuid().primary_key())
                    .col(ColumnDef::new(ClubRoles::ClubId).uuid().not_null())
                    .col(ColumnDef::new(ClubRoles::Name).text().not_null())
                    .col(
                        ColumnDef::new(ClubRoles::IsAdmin)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_club_roles_club")
                            .from(ClubRoles::Table, ClubRoles::ClubId)
                            .to(Clubs::Table, Clubs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_club_roles_club_name")
                    .table(ClubRoles::Table)
                    .col(ClubRoles::ClubId)
                    .col(ClubRoles::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        // Supports the composite FKs below: (club_id, role_id) references
        // must stay inside one club — tenant isolation enforced by the schema.
        manager
            .create_index(
                Index::create()
                    .name("uq_club_roles_club_id_id")
                    .table(ClubRoles::Table)
                    .col(ClubRoles::ClubId)
                    .col(ClubRoles::Id)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ---------- club_role_permissions ----------
        manager
            .create_table(
                Table::create()
                    .table(ClubRolePermissions::Table)
                    .col(
                        ColumnDef::new(ClubRolePermissions::RoleId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ClubRolePermissions::Permission)
                            .text()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ClubRolePermissions::RoleId)
                            .col(ClubRolePermissions::Permission),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_club_role_permissions_role")
                            .from(ClubRolePermissions::Table, ClubRolePermissions::RoleId)
                            .to(ClubRoles::Table, ClubRoles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // ---------- memberships ----------
        let mut memberships = Table::create();
        memberships
            .table(Memberships::Table)
            .col(ColumnDef::new(Memberships::Id).uuid().primary_key())
            .col(ColumnDef::new(Memberships::ClubId).uuid().not_null())
            .col(ColumnDef::new(Memberships::UserId).uuid().not_null())
            .col(ColumnDef::new(Memberships::RoleId).uuid().not_null())
            .col(
                ColumnDef::new(Memberships::Status)
                    .text()
                    .not_null()
                    .check(Expr::col(Memberships::Status).is_in(["active", "suspended"])),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_memberships_club")
                    .from(Memberships::Table, Memberships::ClubId)
                    .to(Clubs::Table, Clubs::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_memberships_user")
                    .from(Memberships::Table, Memberships::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                // Composite FK: a membership can only reference a role of its
                // own club (cross-tenant role assignment is impossible at the
                // database level).
                ForeignKey::create()
                    .name("fk_memberships_club_role")
                    .from(
                        Memberships::Table,
                        (Memberships::ClubId, Memberships::RoleId),
                    )
                    .to(ClubRoles::Table, (ClubRoles::ClubId, ClubRoles::Id))
                    .on_delete(ForeignKeyAction::Restrict),
            );
        timestamps(
            &mut memberships,
            Memberships::CreatedAt,
            Memberships::UpdatedAt,
        );
        manager.create_table(memberships).await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_memberships_club_user")
                    .table(Memberships::Table)
                    .col(Memberships::ClubId)
                    .col(Memberships::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_memberships_user")
                    .table(Memberships::Table)
                    .col(Memberships::UserId)
                    .to_owned(),
            )
            .await?;

        // ---------- club_modules ----------
        manager
            .create_table(
                Table::create()
                    .table(ClubModules::Table)
                    .col(ColumnDef::new(ClubModules::ClubId).uuid().not_null())
                    .col(ColumnDef::new(ClubModules::Module).text().not_null())
                    .col(ColumnDef::new(ClubModules::Enabled).boolean().not_null())
                    .primary_key(
                        Index::create()
                            .col(ClubModules::ClubId)
                            .col(ClubModules::Module),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_club_modules_club")
                            .from(ClubModules::Table, ClubModules::ClubId)
                            .to(Clubs::Table, Clubs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // ---------- user_passwords ----------
        manager
            .create_table(
                Table::create()
                    .table(UserPasswords::Table)
                    .col(ColumnDef::new(UserPasswords::UserId).uuid().primary_key())
                    .col(
                        ColumnDef::new(UserPasswords::PasswordHash)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserPasswords::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_passwords_user")
                            .from(UserPasswords::Table, UserPasswords::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // ---------- user_oidc_identities ----------
        manager
            .create_table(
                Table::create()
                    .table(UserOidcIdentities::Table)
                    .col(ColumnDef::new(UserOidcIdentities::Id).uuid().primary_key())
                    .col(ColumnDef::new(UserOidcIdentities::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserOidcIdentities::Provider)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOidcIdentities::Subject)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOidcIdentities::EmailAtLink)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOidcIdentities::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_oidc_identities_user")
                            .from(UserOidcIdentities::Table, UserOidcIdentities::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_user_oidc_identities_provider_subject")
                    .table(UserOidcIdentities::Table)
                    .col(UserOidcIdentities::Provider)
                    .col(UserOidcIdentities::Subject)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_user_oidc_identities_user_provider")
                    .table(UserOidcIdentities::Table)
                    .col(UserOidcIdentities::UserId)
                    .col(UserOidcIdentities::Provider)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ---------- login_magic_links ----------
        manager
            .create_table(
                Table::create()
                    .table(LoginMagicLinks::Table)
                    .col(ColumnDef::new(LoginMagicLinks::Id).uuid().primary_key())
                    .col(ColumnDef::new(LoginMagicLinks::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(LoginMagicLinks::TokenHash)
                            .binary()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(LoginMagicLinks::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(LoginMagicLinks::ConsumedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(LoginMagicLinks::InvalidatedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(LoginMagicLinks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_login_magic_links_user")
                            .from(LoginMagicLinks::Table, LoginMagicLinks::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_login_magic_links_user_created")
                    .table(LoginMagicLinks::Table)
                    .col(LoginMagicLinks::UserId)
                    .col(LoginMagicLinks::CreatedAt)
                    .to_owned(),
            )
            .await?;
        // Maintenance cleanup filters on expiry.
        manager
            .create_index(
                Index::create()
                    .name("idx_login_magic_links_expires")
                    .table(LoginMagicLinks::Table)
                    .col(LoginMagicLinks::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        // ---------- auth_sessions ----------
        manager
            .create_table(
                Table::create()
                    .table(AuthSessions::Table)
                    .col(ColumnDef::new(AuthSessions::Id).uuid().primary_key())
                    .col(ColumnDef::new(AuthSessions::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(AuthSessions::TokenHash)
                            .binary()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(AuthSessions::ImpersonatedUserId).uuid())
                    // Club-admin impersonation is scoped to that club; NULL =
                    // unscoped (global-admin impersonation or no impersonation).
                    .col(ColumnDef::new(AuthSessions::ImpersonatedClubId).uuid())
                    .col(
                        ColumnDef::new(AuthSessions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuthSessions::LastSeenAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuthSessions::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AuthSessions::RevokedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_auth_sessions_user")
                            .from(AuthSessions::Table, AuthSessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_auth_sessions_impersonated_user")
                            .from(AuthSessions::Table, AuthSessions::ImpersonatedUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_sessions_user")
                    .table(AuthSessions::Table)
                    .col(AuthSessions::UserId)
                    .to_owned(),
            )
            .await?;
        // Maintenance cleanup filters on expiry.
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_sessions_expires")
                    .table(AuthSessions::Table)
                    .col(AuthSessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        // ---------- invitations ----------
        manager
            .create_table(
                Table::create()
                    .table(Invitations::Table)
                    .col(ColumnDef::new(Invitations::Id).uuid().primary_key())
                    .col(
                        ColumnDef::new(Invitations::Email)
                            .text()
                            .not_null()
                            .check(lowercase_check(Invitations::Email)),
                    )
                    .col(
                        ColumnDef::new(Invitations::FirstName)
                            .text()
                            .not_null()
                            .check(Expr::col(Invitations::FirstName).ne("")),
                    )
                    .col(ColumnDef::new(Invitations::LastName).text().not_null())
                    .col(ColumnDef::new(Invitations::ClubId).uuid())
                    .col(ColumnDef::new(Invitations::RoleId).uuid())
                    .col(ColumnDef::new(Invitations::InvitedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(Invitations::TokenHash)
                            .binary()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Invitations::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Invitations::ConsumedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Invitations::RevokedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Invitations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invitations_club")
                            .from(Invitations::Table, Invitations::ClubId)
                            .to(Clubs::Table, Clubs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        // Composite FK (NULLs exempt global-admin invitations):
                        // a club invitation can only grant a role of its club.
                        ForeignKey::create()
                            .name("fk_invitations_club_role")
                            .from(
                                Invitations::Table,
                                (Invitations::ClubId, Invitations::RoleId),
                            )
                            .to(ClubRoles::Table, (ClubRoles::ClubId, ClubRoles::Id))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invitations_invited_by")
                            .from(Invitations::Table, Invitations::InvitedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invitations_email")
                    .table(Invitations::Table)
                    .col(Invitations::Email)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invitations_club")
                    .table(Invitations::Table)
                    .col(Invitations::ClubId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invitations_expires")
                    .table(Invitations::Table)
                    .col(Invitations::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        // ADR-0006 §1 justified raw DDL: partial unique index (inexpressible
        // in the sea-query DSL). At most one PENDING invitation per
        // (email, club); the zero UUID stands in for "platform scope".
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX uq_invitations_pending ON invitations \
                 (email, COALESCE(club_id, '00000000-0000-0000-0000-000000000000')) \
                 WHERE consumed_at IS NULL AND revoked_at IS NULL",
            )
            .await?;

        // ---------- audit_log ----------
        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .col(ColumnDef::new(AuditLog::Id).uuid().primary_key())
                    .col(ColumnDef::new(AuditLog::ActorUserId).uuid().not_null())
                    .col(ColumnDef::new(AuditLog::ImpersonatedUserId).uuid())
                    .col(ColumnDef::new(AuditLog::ClubId).uuid())
                    .col(ColumnDef::new(AuditLog::Action).text().not_null())
                    .col(ColumnDef::new(AuditLog::EntityType).text().not_null())
                    .col(ColumnDef::new(AuditLog::EntityId).uuid())
                    .col(ColumnDef::new(AuditLog::Details).json_binary().not_null())
                    .col(
                        ColumnDef::new(AuditLog::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_audit_log_actor")
                            .from(AuditLog::Table, AuditLog::ActorUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_club_created")
                    .table(AuditLog::Table)
                    .col(AuditLog::ClubId)
                    .col(AuditLog::CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_actor_created")
                    .table(AuditLog::Table)
                    .col(AuditLog::ActorUserId)
                    .col(AuditLog::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            AuditLog::Table.into_iden(),
            Invitations::Table.into_iden(),
            AuthSessions::Table.into_iden(),
            LoginMagicLinks::Table.into_iden(),
            UserOidcIdentities::Table.into_iden(),
            UserPasswords::Table.into_iden(),
            ClubModules::Table.into_iden(),
            Memberships::Table.into_iden(),
            ClubRolePermissions::Table.into_iden(),
            ClubRoles::Table.into_iden(),
            Clubs::Table.into_iden(),
            Users::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
    FirstName,
    LastName,
    Status,
    IsGlobalAdmin,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserPasswords {
    Table,
    UserId,
    PasswordHash,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserOidcIdentities {
    Table,
    Id,
    UserId,
    Provider,
    Subject,
    EmailAtLink,
    CreatedAt,
}

#[derive(DeriveIden)]
enum LoginMagicLinks {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    ConsumedAt,
    InvalidatedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AuthSessions {
    Table,
    Id,
    UserId,
    TokenHash,
    ImpersonatedUserId,
    ImpersonatedClubId,
    CreatedAt,
    LastSeenAt,
    ExpiresAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum Invitations {
    Table,
    Id,
    Email,
    FirstName,
    LastName,
    ClubId,
    RoleId,
    InvitedBy,
    TokenHash,
    ExpiresAt,
    ConsumedAt,
    RevokedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    ActorUserId,
    ImpersonatedUserId,
    ClubId,
    Action,
    EntityType,
    EntityId,
    Details,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Clubs {
    Table,
    Id,
    Name,
    Slug,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ClubRoles {
    Table,
    Id,
    ClubId,
    Name,
    IsAdmin,
}

#[derive(DeriveIden)]
enum ClubRolePermissions {
    Table,
    RoleId,
    Permission,
}

#[derive(DeriveIden)]
enum Memberships {
    Table,
    Id,
    ClubId,
    UserId,
    RoleId,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ClubModules {
    Table,
    ClubId,
    Module,
    Enabled,
}
