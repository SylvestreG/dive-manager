use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "club_role_permissions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub permission: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::club_roles::Entity",
        from = "Column::RoleId",
        to = "super::club_roles::Column::Id"
    )]
    Role,
}

impl Related<super::club_roles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
