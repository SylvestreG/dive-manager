use sea_orm::entity::prelude::*;

/// Instructor stamp image (E2+), separate table so profile reads never load
/// the blob.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_stamps")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub image: Vec<u8>,
    pub mime_type: String,
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
