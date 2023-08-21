use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::problem;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "contests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub owner_id: i32,
    #[sea_orm(column_type = "Timestamp")]
    pub begin: DateTime,
    #[sea_orm(column_type = "Timestamp")]
    pub end: DateTime,
    pub description: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    // User,
    Problem,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            // Self::User => Entity::belongs_to(user::Entity)
            //     .from(Column::UserId)
            //     .to(user::Column::Id)
            //     .into(),
            Self::Problem => Entity::has_many(problem::Entity).into(),
        }
    }
}

// impl Related<user::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::User.def()
//     }
// }

impl Related<problem::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Problem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
