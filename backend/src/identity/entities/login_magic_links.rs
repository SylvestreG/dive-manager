use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "login_magic_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: Vec<u8>,
    pub expires_at: DateTimeWithTimeZone,
    pub consumed_at: Option<DateTimeWithTimeZone>,
    pub invalidated_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    /// `login` | `confirm_link` | `confirm_email` (ADR-0009).
    pub purpose: String,
    /// confirm_link: {provider, subject, email}; confirm_email: {new_email}.
    pub payload: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}

pub const PURPOSE_LOGIN: &str = "login";
pub const PURPOSE_CONFIRM_LINK: &str = "confirm_link";
pub const PURPOSE_CONFIRM_EMAIL: &str = "confirm_email";
