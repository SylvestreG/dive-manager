use sea_orm::entity::prelude::*;

/// Stamp box of one skill on the level's PDF template. Coordinates in PDF
/// points, origin bottom-left, (x, y) = bottom-left corner of the box.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "level_document_positions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub level_document_id: Uuid,
    pub skill_id: Uuid,
    /// 1-based page number.
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::level_documents::Entity",
        from = "Column::LevelDocumentId",
        to = "super::level_documents::Column::Id"
    )]
    Document,
    #[sea_orm(
        belongs_to = "super::competency_skills::Entity",
        from = "Column::SkillId",
        to = "super::competency_skills::Column::Id"
    )]
    Skill,
}

impl ActiveModelBehavior for ActiveModel {}
