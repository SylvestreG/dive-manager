use sea_orm::entity::prelude::*;

/// Invitation to join the platform: a club with a role (`club_id` set), or as
/// a global administrator (`club_id` NULL).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "invitations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub club_id: Option<Uuid>,
    pub role_id: Option<Uuid>,
    pub invited_by: Uuid,
    pub token_hash: Vec<u8>,
    pub expires_at: DateTimeWithTimeZone,
    pub consumed_at: Option<DateTimeWithTimeZone>,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
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
        belongs_to = "super::club_roles::Entity",
        from = "Column::RoleId",
        to = "super::club_roles::Column::Id"
    )]
    Role,
}

impl ActiveModelBehavior for ActiveModel {}
