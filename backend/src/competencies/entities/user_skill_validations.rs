use sea_orm::entity::prelude::*;

/// Current validation state: one row per (user, skill). Every write also
/// appends to `user_skill_validation_events` — this table is never the
/// history, only the present.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_skill_validations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub skill_id: Uuid,
    /// Fixed stage code: "vu" | "a_perfectionner" | "valide".
    pub stage: String,
    pub validated_at: Date,
    pub validated_by_user_id: Option<Uuid>,
    pub validated_in_club_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::competency_skills::Entity",
        from = "Column::SkillId",
        to = "super::competency_skills::Column::Id"
    )]
    Skill,
    #[sea_orm(
        belongs_to = "crate::identity::entities::users::Entity",
        from = "Column::UserId",
        to = "crate::identity::entities::users::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
