//! OIDC login — slice 2 of the auth & onboarding redesign
//! (docs/design/auth-onboarding.md §7, ADR-0009):
//! - `user_oidc_identities` becomes `login_identities` (provider email
//!   optional, `last_used_at`);
//! - `oidc_flows`: server-side state of one authorization round-trip
//!   (state, nonce, PKCE verifier) — 10 minutes;
//! - `exchange_codes`: one-time 60 s codes the web page / app trades for a
//!   session, so no session token ever travels in a URL;
//! - `login_magic_links.purpose` + `payload` (confirm an identity link,
//!   confirm an email change);
//! - `auth_sessions.client_label` ("My devices").

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .rename_table(
                Table::rename()
                    .table(UserOidcIdentities::Table, LoginIdentities::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(LoginIdentities::Table)
                    .modify_column(ColumnDef::new(LoginIdentities::EmailAtLink).text().null())
                    .add_column(
                        ColumnDef::new(LoginIdentities::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OidcFlows::Table)
                    .col(ColumnDef::new(OidcFlows::Id).uuid().primary_key())
                    .col(
                        ColumnDef::new(OidcFlows::StateHash)
                            .binary()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(OidcFlows::Provider).text().not_null())
                    .col(
                        ColumnDef::new(OidcFlows::Client)
                            .text()
                            .not_null()
                            .check(Expr::col(OidcFlows::Client).is_in(["web", "ios"])),
                    )
                    .col(
                        ColumnDef::new(OidcFlows::Purpose)
                            .text()
                            .not_null()
                            .check(Expr::col(OidcFlows::Purpose).is_in(["login", "link"])),
                    )
                    .col(ColumnDef::new(OidcFlows::Nonce).text().not_null())
                    .col(ColumnDef::new(OidcFlows::PkceVerifier).text().not_null())
                    .col(ColumnDef::new(OidcFlows::LinkUserId).uuid().null())
                    .col(
                        ColumnDef::new(OidcFlows::ConsumedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OidcFlows::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OidcFlows::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .check(
                        // A link flow is bound to the user who started it.
                        Expr::col(OidcFlows::Purpose)
                            .eq("login")
                            .and(Expr::col(OidcFlows::LinkUserId).is_null())
                            .or(Expr::col(OidcFlows::Purpose)
                                .eq("link")
                                .and(Expr::col(OidcFlows::LinkUserId).is_not_null())),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oidc_flows_link_user")
                            .from(OidcFlows::Table, OidcFlows::LinkUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_oidc_flows_expires")
                    .table(OidcFlows::Table)
                    .col(OidcFlows::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ExchangeCodes::Table)
                    .col(
                        ColumnDef::new(ExchangeCodes::CodeHash)
                            .binary()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ExchangeCodes::UserId).uuid().not_null())
                    .col(ColumnDef::new(ExchangeCodes::ClientLabel).text().null())
                    .col(
                        ColumnDef::new(ExchangeCodes::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExchangeCodes::ConsumedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_exchange_codes_user")
                            .from(ExchangeCodes::Table, ExchangeCodes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_exchange_codes_expires")
                    .table(ExchangeCodes::Table)
                    .col(ExchangeCodes::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LoginMagicLinks::Table)
                    .add_column(
                        ColumnDef::new(LoginMagicLinks::Purpose)
                            .text()
                            .not_null()
                            .default("login")
                            .check(Expr::col(LoginMagicLinks::Purpose).is_in([
                                "login",
                                "confirm_link",
                                "confirm_email",
                            ])),
                    )
                    .add_column(
                        ColumnDef::new(LoginMagicLinks::Payload)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(AuthSessions::Table)
                    .add_column(ColumnDef::new(AuthSessions::ClientLabel).text().null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AuthSessions::Table)
                    .drop_column(AuthSessions::ClientLabel)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(LoginMagicLinks::Table)
                    .drop_column(LoginMagicLinks::Purpose)
                    .drop_column(LoginMagicLinks::Payload)
                    .to_owned(),
            )
            .await?;
        for table in [
            ExchangeCodes::Table.into_iden(),
            OidcFlows::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(LoginIdentities::Table)
                    .drop_column(LoginIdentities::LastUsedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .rename_table(
                Table::rename()
                    .table(LoginIdentities::Table, UserOidcIdentities::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum UserOidcIdentities {
    Table,
}

#[derive(DeriveIden)]
enum LoginIdentities {
    Table,
    EmailAtLink,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum OidcFlows {
    Table,
    Id,
    StateHash,
    Provider,
    Client,
    Purpose,
    Nonce,
    PkceVerifier,
    LinkUserId,
    ConsumedAt,
    ExpiresAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ExchangeCodes {
    Table,
    CodeHash,
    UserId,
    ClientLabel,
    ExpiresAt,
    ConsumedAt,
}

#[derive(DeriveIden)]
enum LoginMagicLinks {
    Table,
    Purpose,
    Payload,
}

#[derive(DeriveIden)]
enum AuthSessions {
    Table,
    ClientLabel,
}
