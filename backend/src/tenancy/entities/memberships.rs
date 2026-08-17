use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "memberships")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub club_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clubs::Entity",
        from = "Column::ClubId",
        to = "super::clubs::Column::Id"
    )]
    Club,
    #[sea_orm(
        belongs_to = "crate::identity::entities::users::Entity",
        from = "Column::UserId",
        to = "crate::identity::entities::users::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::club_roles::Entity",
        from = "Column::RoleId",
        to = "super::club_roles::Column::Id"
    )]
    Role,
}

impl Related<crate::identity::entities::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::clubs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Club.def()
    }
}

impl Related<super::club_roles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_SUSPENDED: &str = "suspended";
