use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_training_statuses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub valid_until: Date,
    pub created_at: DateTimeWithTimeZone,
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

pub const STATUS_STAGIAIRE_MF1: &str = "stagiaire_mf1";
