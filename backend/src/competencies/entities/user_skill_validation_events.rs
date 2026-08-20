use sea_orm::entity::prelude::*;

/// Append-only validation history. Rows are inserted, never updated or
/// deleted; skill/level data is denormalized so the history survives tree
/// restructuring (deleted skills SET NULL the FK but keep the names).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_skill_validation_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub skill_id: Option<Uuid>,
    pub skill_name: String,
    pub certification: String,
    /// "validated" | "reverted" | "deleted".
    pub action: String,
    /// Target stage (None for deleted).
    pub stage: Option<String>,
    pub previous_stage: Option<String>,
    pub validated_at: Option<Date>,
    pub performed_by_user_id: Option<Uuid>,
    pub performed_in_club_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub const ACTION_VALIDATED: &str = "validated";
pub const ACTION_REVERTED: &str = "reverted";
pub const ACTION_DELETED: &str = "deleted";
