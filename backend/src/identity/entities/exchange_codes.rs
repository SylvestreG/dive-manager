use sea_orm::entity::prelude::*;

/// One-time code (60 s) traded for a session by the page or app that
/// receives the OIDC redirect — session tokens never travel in URLs.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "exchange_codes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub code_hash: Vec<u8>,
    pub user_id: Uuid,
    pub client_label: Option<String>,
    pub expires_at: DateTimeWithTimeZone,
    pub consumed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
