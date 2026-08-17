use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "clubs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::memberships::Entity")]
    Memberships,
}

impl Related<super::memberships::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Memberships.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_SUSPENDED: &str = "suspended";
