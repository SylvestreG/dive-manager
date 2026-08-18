use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub birthdate: Option<Date>,
    pub phone: Option<String>,
    pub licence_number: Option<String>,
    pub instructor_number: Option<String>,
    pub medical_certificate_valid_until: Option<Date>,
    pub gear_regulator: bool,
    pub gear_stab: bool,
    pub gear_stab_size: Option<String>,
    pub gear_nitrox: bool,
    pub gear_second_regulator: bool,
    /// ffessm certification code being prepared (one at a time).
    pub preparing_certification: Option<String>,
    /// Handisub pathway (situation de handicap) — gates PESH suggestions.
    pub handisub: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::identity::entities::users::Entity",
        from = "Column::UserId",
        to = "crate::identity::entities::users::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
