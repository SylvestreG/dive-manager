use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub status: String,
    pub is_global_admin: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::user_passwords::Entity")]
    Password,
}

impl Related<super::user_passwords::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Password.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub const STATUS_INVITED: &str = "invited";
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_DISABLED: &str = "disabled";
