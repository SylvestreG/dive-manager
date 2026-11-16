use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Optional (members imported without an address); unique when present,
    /// lowercase. Magic links are sent here.
    pub email: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub status: String,
    pub is_global_admin: bool,
    /// First successful login, any method. NULL = never logged in.
    pub activated_at: Option<DateTimeWithTimeZone>,
    /// Set when ownership of `email` was proven (magic link clicked).
    pub email_verified_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_DISABLED: &str = "disabled";
