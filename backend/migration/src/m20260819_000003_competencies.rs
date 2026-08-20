//! `competencies` module (docs/design/competencies-module.md).
//! The competency tree is PLATFORM-GLOBAL (product decision 2026-08-19) and
//! validations are USER-GLOBAL: no table here carries a club_id as ownership —
//! clubs only appear as attribution (where a validation happened).
//! Validation stages are hardcoded (vu / a_perfectionner / valide): no stage
//! table, the CHECK constraints below freeze the closed set.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const STAGES: [&str; 3] = ["vu", "a_perfectionner", "valide"];

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

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut domains = Table::create()
            .table(CompetencyDomains::Table)
            .col(ColumnDef::new(CompetencyDomains::Id).uuid().primary_key())
            // ffessm certification code — validated by the application
            // (single source of truth is the ffessm crate's golden list).
            .col(
                ColumnDef::new(CompetencyDomains::Certification)
                    .text()
                    .not_null(),
            )
            .col(
                ColumnDef::new(CompetencyDomains::Name)
                    .text()
                    .not_null()
                    .check(Expr::col(CompetencyDomains::Name).ne("")),
            )
            .col(
                ColumnDef::new(CompetencyDomains::SortOrder)
                    .integer()
                    .not_null(),
            )
            .to_owned();
        timestamps(
            &mut domains,
            CompetencyDomains::CreatedAt,
            CompetencyDomains::UpdatedAt,
        );
        manager.create_table(domains).await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_competency_domains_cert_name")
                    .table(CompetencyDomains::Table)
                    .col(CompetencyDomains::Certification)
                    .col(CompetencyDomains::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_competency_domains_certification")
                    .table(CompetencyDomains::Table)
                    .col(CompetencyDomains::Certification)
                    .to_owned(),
            )
            .await?;

        let mut modules = Table::create()
            .table(CompetencyModules::Table)
            .col(ColumnDef::new(CompetencyModules::Id).uuid().primary_key())
            .col(
                ColumnDef::new(CompetencyModules::DomainId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(CompetencyModules::Name)
                    .text()
                    .not_null()
                    .check(Expr::col(CompetencyModules::Name).ne("")),
            )
            .col(
                ColumnDef::new(CompetencyModules::SortOrder)
                    .integer()
                    .not_null(),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_competency_modules_domain")
                    .from(CompetencyModules::Table, CompetencyModules::DomainId)
                    .to(CompetencyDomains::Table, CompetencyDomains::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned();
        timestamps(
            &mut modules,
            CompetencyModules::CreatedAt,
            CompetencyModules::UpdatedAt,
        );
        manager.create_table(modules).await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_competency_modules_domain_name")
                    .table(CompetencyModules::Table)
                    .col(CompetencyModules::DomainId)
                    .col(CompetencyModules::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        // Postgres does not index FK columns.
        manager
            .create_index(
                Index::create()
                    .name("idx_competency_modules_domain_id")
                    .table(CompetencyModules::Table)
                    .col(CompetencyModules::DomainId)
                    .to_owned(),
            )
            .await?;

        let mut skills = Table::create()
            .table(CompetencySkills::Table)
            .col(ColumnDef::new(CompetencySkills::Id).uuid().primary_key())
            .col(ColumnDef::new(CompetencySkills::ModuleId).uuid().not_null())
            .col(
                ColumnDef::new(CompetencySkills::Name)
                    .text()
                    .not_null()
                    .check(Expr::col(CompetencySkills::Name).ne("")),
            )
            .col(ColumnDef::new(CompetencySkills::Description).text())
            .col(
                ColumnDef::new(CompetencySkills::SortOrder)
                    .integer()
                    .not_null(),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_competency_skills_module")
                    .from(CompetencySkills::Table, CompetencySkills::ModuleId)
                    .to(CompetencyModules::Table, CompetencyModules::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned();
        timestamps(
            &mut skills,
            CompetencySkills::CreatedAt,
            CompetencySkills::UpdatedAt,
        );
        manager.create_table(skills).await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_competency_skills_module_name")
                    .table(CompetencySkills::Table)
                    .col(CompetencySkills::ModuleId)
                    .col(CompetencySkills::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_competency_skills_module_id")
                    .table(CompetencySkills::Table)
                    .col(CompetencySkills::ModuleId)
                    .to_owned(),
            )
            .await?;

        // Current validation state: exactly one row per (user, skill). The
        // full history lives in user_skill_validation_events.
        let mut validations = Table::create()
            .table(UserSkillValidations::Table)
            .col(
                ColumnDef::new(UserSkillValidations::Id)
                    .uuid()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(UserSkillValidations::UserId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(UserSkillValidations::SkillId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(UserSkillValidations::Stage)
                    .text()
                    .not_null()
                    .check(Expr::col(UserSkillValidations::Stage).is_in(STAGES)),
            )
            .col(
                ColumnDef::new(UserSkillValidations::ValidatedAt)
                    .date()
                    .not_null(),
            )
            .col(ColumnDef::new(UserSkillValidations::ValidatedByUserId).uuid())
            .col(ColumnDef::new(UserSkillValidations::ValidatedInClubId).uuid())
            .col(ColumnDef::new(UserSkillValidations::Notes).text())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_user_skill_validations_user")
                    .from(UserSkillValidations::Table, UserSkillValidations::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_user_skill_validations_skill")
                    .from(UserSkillValidations::Table, UserSkillValidations::SkillId)
                    .to(CompetencySkills::Table, CompetencySkills::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_user_skill_validations_validated_by")
                    .from(
                        UserSkillValidations::Table,
                        UserSkillValidations::ValidatedByUserId,
                    )
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::SetNull),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_user_skill_validations_validated_in_club")
                    .from(
                        UserSkillValidations::Table,
                        UserSkillValidations::ValidatedInClubId,
                    )
                    .to(Clubs::Table, Clubs::Id)
                    .on_delete(ForeignKeyAction::SetNull),
            )
            .to_owned();
        timestamps(
            &mut validations,
            UserSkillValidations::CreatedAt,
            UserSkillValidations::UpdatedAt,
        );
        manager.create_table(validations).await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_user_skill_validations_user_skill")
                    .table(UserSkillValidations::Table)
                    .col(UserSkillValidations::UserId)
                    .col(UserSkillValidations::SkillId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        for (name, col) in [
            ("idx_user_skill_validations_skill_id", {
                UserSkillValidations::SkillId
            }),
            (
                "idx_user_skill_validations_validated_by",
                UserSkillValidations::ValidatedByUserId,
            ),
            (
                "idx_user_skill_validations_validated_in_club",
                UserSkillValidations::ValidatedInClubId,
            ),
        ] {
            manager
                .create_index(
                    Index::create()
                        .name(name)
                        .table(UserSkillValidations::Table)
                        .col(col)
                        .to_owned(),
                )
                .await?;
        }

        // Append-only history. Skill/stage data is denormalized so events
        // survive tree restructuring; rows are never updated or deleted.
        manager
            .create_table(
                Table::create()
                    .table(UserSkillValidationEvents::Table)
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::Id)
                            .uuid()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UserSkillValidationEvents::SkillId).uuid())
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::SkillName)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::Certification)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::Action)
                            .text()
                            .not_null()
                            .check(Expr::col(UserSkillValidationEvents::Action).is_in([
                                "validated",
                                "reverted",
                                "deleted",
                            ])),
                    )
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::Stage)
                            .text()
                            .check(Expr::col(UserSkillValidationEvents::Stage).is_in(STAGES)),
                    )
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::PreviousStage)
                            .text()
                            .check(
                                Expr::col(UserSkillValidationEvents::PreviousStage).is_in(STAGES),
                            ),
                    )
                    .col(ColumnDef::new(UserSkillValidationEvents::ValidatedAt).date())
                    .col(ColumnDef::new(UserSkillValidationEvents::PerformedByUserId).uuid())
                    .col(ColumnDef::new(UserSkillValidationEvents::PerformedInClubId).uuid())
                    .col(ColumnDef::new(UserSkillValidationEvents::Notes).text())
                    .col(
                        ColumnDef::new(UserSkillValidationEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_usv_events_user")
                            .from(
                                UserSkillValidationEvents::Table,
                                UserSkillValidationEvents::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_usv_events_skill")
                            .from(
                                UserSkillValidationEvents::Table,
                                UserSkillValidationEvents::SkillId,
                            )
                            .to(CompetencySkills::Table, CompetencySkills::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_usv_events_performed_by")
                            .from(
                                UserSkillValidationEvents::Table,
                                UserSkillValidationEvents::PerformedByUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_usv_events_performed_in_club")
                            .from(
                                UserSkillValidationEvents::Table,
                                UserSkillValidationEvents::PerformedInClubId,
                            )
                            .to(Clubs::Table, Clubs::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_usv_events_user_id_created_at")
                    .table(UserSkillValidationEvents::Table)
                    .col(UserSkillValidationEvents::UserId)
                    .col(UserSkillValidationEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;
        for (name, col) in [
            (
                "idx_usv_events_skill_id",
                UserSkillValidationEvents::SkillId,
            ),
            (
                "idx_usv_events_performed_by",
                UserSkillValidationEvents::PerformedByUserId,
            ),
            (
                "idx_usv_events_performed_in_club",
                UserSkillValidationEvents::PerformedInClubId,
            ),
        ] {
            manager
                .create_index(
                    Index::create()
                        .name(name)
                        .table(UserSkillValidationEvents::Table)
                        .col(col)
                        .to_owned(),
                )
                .await?;
        }

        // One global PDF template per certification, blob kept in-row (the
        // user_stamps precedent) — list reads must never select FileData.
        let mut documents = Table::create()
            .table(LevelDocuments::Table)
            .col(ColumnDef::new(LevelDocuments::Id).uuid().primary_key())
            .col(
                ColumnDef::new(LevelDocuments::Certification)
                    .text()
                    .not_null()
                    .unique_key(),
            )
            .col(ColumnDef::new(LevelDocuments::FileName).text().not_null())
            .col(ColumnDef::new(LevelDocuments::FileData).binary().not_null())
            .col(
                ColumnDef::new(LevelDocuments::PageCount)
                    .integer()
                    .not_null(),
            )
            .to_owned();
        timestamps(
            &mut documents,
            LevelDocuments::CreatedAt,
            LevelDocuments::UpdatedAt,
        );
        manager.create_table(documents).await?;

        // Stamp boxes, in PDF points (origin bottom-left, bottom-left anchor,
        // Fosse's model). Real FK to the document row (Fosse linked by a bare
        // string and orphaned positions).
        let mut positions = Table::create()
            .table(LevelDocumentPositions::Table)
            .col(
                ColumnDef::new(LevelDocumentPositions::Id)
                    .uuid()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(LevelDocumentPositions::LevelDocumentId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(LevelDocumentPositions::SkillId)
                    .uuid()
                    .not_null(),
            )
            .col(
                ColumnDef::new(LevelDocumentPositions::Page)
                    .integer()
                    .not_null()
                    .check(Expr::col(LevelDocumentPositions::Page).gte(1)),
            )
            .col(ColumnDef::new(LevelDocumentPositions::X).float().not_null())
            .col(ColumnDef::new(LevelDocumentPositions::Y).float().not_null())
            .col(
                ColumnDef::new(LevelDocumentPositions::Width)
                    .float()
                    .not_null(),
            )
            .col(
                ColumnDef::new(LevelDocumentPositions::Height)
                    .float()
                    .not_null(),
            )
            .col(
                ColumnDef::new(LevelDocumentPositions::FontSize)
                    .float()
                    .not_null()
                    .default(8.0),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_level_document_positions_document")
                    .from(
                        LevelDocumentPositions::Table,
                        LevelDocumentPositions::LevelDocumentId,
                    )
                    .to(LevelDocuments::Table, LevelDocuments::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_level_document_positions_skill")
                    .from(
                        LevelDocumentPositions::Table,
                        LevelDocumentPositions::SkillId,
                    )
                    .to(CompetencySkills::Table, CompetencySkills::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned();
        timestamps(
            &mut positions,
            LevelDocumentPositions::CreatedAt,
            LevelDocumentPositions::UpdatedAt,
        );
        manager.create_table(positions).await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_level_document_positions_document_skill")
                    .table(LevelDocumentPositions::Table)
                    .col(LevelDocumentPositions::LevelDocumentId)
                    .col(LevelDocumentPositions::SkillId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_level_document_positions_skill_id")
                    .table(LevelDocumentPositions::Table)
                    .col(LevelDocumentPositions::SkillId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            LevelDocumentPositions::Table.into_iden(),
            LevelDocuments::Table.into_iden(),
            UserSkillValidationEvents::Table.into_iden(),
            UserSkillValidations::Table.into_iden(),
            CompetencySkills::Table.into_iden(),
            CompetencyModules::Table.into_iden(),
            CompetencyDomains::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CompetencyDomains {
    Table,
    Id,
    Certification,
    Name,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum CompetencyModules {
    Table,
    Id,
    DomainId,
    Name,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum CompetencySkills {
    Table,
    Id,
    ModuleId,
    Name,
    Description,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserSkillValidations {
    Table,
    Id,
    UserId,
    SkillId,
    Stage,
    ValidatedAt,
    ValidatedByUserId,
    ValidatedInClubId,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserSkillValidationEvents {
    Table,
    Id,
    UserId,
    SkillId,
    SkillName,
    Certification,
    Action,
    Stage,
    PreviousStage,
    ValidatedAt,
    PerformedByUserId,
    PerformedInClubId,
    Notes,
    CreatedAt,
}

#[derive(DeriveIden)]
enum LevelDocuments {
    Table,
    Id,
    Certification,
    FileName,
    FileData,
    PageCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum LevelDocumentPositions {
    Table,
    Id,
    LevelDocumentId,
    SkillId,
    Page,
    X,
    Y,
    Width,
    Height,
    FontSize,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Clubs {
    Table,
    Id,
}
