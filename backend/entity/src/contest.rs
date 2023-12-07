//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "contest")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub hoster: i32,
    #[sea_orm(column_type = "Time")]
    pub begin: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time")]
    pub end: chrono::NaiveDateTime,
    pub title: String,
    pub content: String,
    pub tags: String,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))", nullable)]
    pub password: Option<Vec<u8>>,
    #[sea_orm(column_type = "Time")]
    pub create_at: chrono::NaiveDateTime,
    #[sea_orm(column_type = "Time", on_update = "current_timestamp")]
    pub update_at: chrono::NaiveDateTime,
    pub public: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_contest::Entity")]
    UserContest,
    #[sea_orm(has_one = "super::problem::Entity")]
    Problem,
}

impl Related<super::user_contest::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserContest.def()
    }
}

impl Related<super::problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_contest::Relation::Contest.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_contest::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
