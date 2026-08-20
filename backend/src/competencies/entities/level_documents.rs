use sea_orm::entity::prelude::*;

/// Global PDF template of a level's competency document. The blob lives
/// in-row (user_stamps precedent): list reads must project WITHOUT FileData.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "level_documents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// ffessm certification code, one template per level.
    pub certification: String,
    pub file_name: String,
    pub file_data: Vec<u8>,
    pub page_count: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
