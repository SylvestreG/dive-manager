//! `users` module: global diving profile (docs/design/users-module.md).
//! The profile belongs to the PERSON — certifications are federal and follow
//! the diver across clubs; clubs get visibility through active memberships.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserProfiles::Table)
                    .col(ColumnDef::new(UserProfiles::UserId).uuid().primary_key())
                    .col(ColumnDef::new(UserProfiles::Birthdate).date())
                    .col(ColumnDef::new(UserProfiles::Phone).text())
                    .col(ColumnDef::new(UserProfiles::LicenceNumber).text())
                    .col(ColumnDef::new(UserProfiles::InstructorNumber).text())
                    .col(ColumnDef::new(UserProfiles::MedicalCertificateValidUntil).date())
                    .col(
                        ColumnDef::new(UserProfiles::GearRegulator)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::GearStab)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(UserProfiles::GearStabSize).text().check(
                        Expr::col(UserProfiles::GearStabSize).is_in(["XS", "S", "M", "L", "XL"]),
                    ))
                    .col(
                        ColumnDef::new(UserProfiles::GearNitrox)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::GearSecondRegulator)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(UserProfiles::PreparingCertification).text())
                    // Handisub pathway (situation de handicap): gates the
                    // PESH certifications in the derived suggestions.
                    .col(
                        ColumnDef::new(UserProfiles::Handisub)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserProfiles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_profiles_user")
                            .from(UserProfiles::Table, UserProfiles::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserCertifications::Table)
                    .col(ColumnDef::new(UserCertifications::Id).uuid().primary_key())
                    .col(ColumnDef::new(UserCertifications::UserId).uuid().not_null())
                    // ffessm code — validated by the application (single
                    // source of truth is the ffessm crate's golden list).
                    .col(
                        ColumnDef::new(UserCertifications::Certification)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UserCertifications::ObtainedOn).date())
                    .col(
                        ColumnDef::new(UserCertifications::Provenance)
                            .text()
                            .not_null()
                            .check(Expr::col(UserCertifications::Provenance).is_in([
                                "declared",
                                "recorded",
                                "imported",
                                "validated",
                            ])),
                    )
                    .col(ColumnDef::new(UserCertifications::RecordedByUserId).uuid())
                    .col(ColumnDef::new(UserCertifications::RecordedInClubId).uuid())
                    .col(
                        ColumnDef::new(UserCertifications::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserCertifications::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_certifications_user")
                            .from(UserCertifications::Table, UserCertifications::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_certifications_recorded_by")
                            .from(
                                UserCertifications::Table,
                                UserCertifications::RecordedByUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_certifications_recorded_in_club")
                            .from(
                                UserCertifications::Table,
                                UserCertifications::RecordedInClubId,
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
                    .name("uq_user_certifications_user_certification")
                    .table(UserCertifications::Table)
                    .col(UserCertifications::UserId)
                    .col(UserCertifications::Certification)
                    .unique()
                    .to_owned(),
            )
            .await?;
        // Postgres does not index FK columns: without these, every user or
        // club deletion (SET NULL) scans the whole table.
        manager
            .create_index(
                Index::create()
                    .name("idx_user_certifications_recorded_by")
                    .table(UserCertifications::Table)
                    .col(UserCertifications::RecordedByUserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_user_certifications_recorded_in_club")
                    .table(UserCertifications::Table)
                    .col(UserCertifications::RecordedInClubId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserTrainingStatuses::Table)
                    .col(
                        ColumnDef::new(UserTrainingStatuses::Id)
                            .uuid()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserTrainingStatuses::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserTrainingStatuses::Status)
                            .text()
                            .not_null()
                            .check(
                                Expr::col(UserTrainingStatuses::Status).is_in(["stagiaire_mf1"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(UserTrainingStatuses::ValidUntil)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserTrainingStatuses::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_training_statuses_user")
                            .from(UserTrainingStatuses::Table, UserTrainingStatuses::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_user_training_statuses_user_status")
                    .table(UserTrainingStatuses::Table)
                    .col(UserTrainingStatuses::UserId)
                    .col(UserTrainingStatuses::Status)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Instructor stamp image (E2+), kept apart so profile reads never
        // load the blob.
        manager
            .create_table(
                Table::create()
                    .table(UserStamps::Table)
                    .col(ColumnDef::new(UserStamps::UserId).uuid().primary_key())
                    .col(ColumnDef::new(UserStamps::Image).binary().not_null())
                    .col(
                        ColumnDef::new(UserStamps::MimeType)
                            .text()
                            .not_null()
                            .check(
                                Expr::col(UserStamps::MimeType).is_in(["image/png", "image/jpeg"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(UserStamps::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_stamps_user")
                            .from(UserStamps::Table, UserStamps::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            UserStamps::Table.into_iden(),
            UserTrainingStatuses::Table.into_iden(),
            UserCertifications::Table.into_iden(),
            UserProfiles::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum UserProfiles {
    Table,
    UserId,
    Birthdate,
    Phone,
    LicenceNumber,
    InstructorNumber,
    MedicalCertificateValidUntil,
    GearRegulator,
    GearStab,
    GearStabSize,
    GearNitrox,
    GearSecondRegulator,
    PreparingCertification,
    Handisub,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserCertifications {
    Table,
    Id,
    UserId,
    Certification,
    ObtainedOn,
    Provenance,
    RecordedByUserId,
    RecordedInClubId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserTrainingStatuses {
    Table,
    Id,
    UserId,
    Status,
    ValidUntil,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserStamps {
    Table,
    UserId,
    Image,
    MimeType,
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
