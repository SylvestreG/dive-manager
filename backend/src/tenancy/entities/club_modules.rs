use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "club_modules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub club_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub module: String,
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clubs::Entity",
        from = "Column::ClubId",
        to = "super::clubs::Column::Id"
    )]
    Club,
}

impl ActiveModelBehavior for ActiveModel {}
