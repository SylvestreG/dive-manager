use sea_orm::entity::prelude::*;

/// Server-side state of one OIDC authorization round-trip (10 minutes).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "oidc_flows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub state_hash: Vec<u8>,
    pub provider: String,
    pub client: String,
    pub purpose: String,
    pub nonce: String,
    pub pkce_verifier: String,
    /// Set for `purpose = link`: the logged-in user who started the flow.
    pub link_user_id: Option<Uuid>,
    pub consumed_at: Option<DateTimeWithTimeZone>,
    pub expires_at: DateTimeWithTimeZone,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub const PURPOSE_LOGIN: &str = "login";
pub const PURPOSE_LINK: &str = "link";
pub const CLIENT_WEB: &str = "web";
pub const CLIENT_IOS: &str = "ios";
