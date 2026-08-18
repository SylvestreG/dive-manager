use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_certifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    /// ffessm certification code (frozen golden list).
    pub certification: String,
    pub obtained_on: Option<Date>,
    /// Who asserts it: declared < recorded = imported < validated.
    pub provenance: String,
    pub recorded_by_user_id: Option<Uuid>,
    pub recorded_in_club_id: Option<Uuid>,
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

pub const PROVENANCE_DECLARED: &str = "declared";
pub const PROVENANCE_RECORDED: &str = "recorded";
pub const PROVENANCE_IMPORTED: &str = "imported";
pub const PROVENANCE_VALIDATED: &str = "validated";

/// Trust rank: a new write may only keep-or-raise it (business rule 2).
pub fn provenance_rank(provenance: &str) -> u8 {
    match provenance {
        PROVENANCE_DECLARED => 0,
        PROVENANCE_RECORDED | PROVENANCE_IMPORTED => 1,
        PROVENANCE_VALIDATED => 2,
        _ => 0,
    }
}
