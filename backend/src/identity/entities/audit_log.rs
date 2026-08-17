use sea_orm::entity::prelude::*;

/// Audit trail. `club_id` is a plain scoping value (no FK) so the audit log
/// stays module-independent and survives club deletion.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub actor_user_id: Uuid,
    pub impersonated_user_id: Option<Uuid>,
    pub club_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    #[sea_orm(column_type = "JsonBinary")]
    pub details: Json,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
