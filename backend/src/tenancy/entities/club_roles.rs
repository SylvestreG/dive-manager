use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "club_roles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub club_id: Uuid,
    pub name: String,
    pub is_admin: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clubs::Entity",
        from = "Column::ClubId",
        to = "super::clubs::Column::Id"
    )]
    Club,
    #[sea_orm(has_many = "super::club_role_permissions::Entity")]
    Permissions,
}

impl Related<super::club_role_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Permissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
