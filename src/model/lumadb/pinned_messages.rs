//! SeaORM Entity. Generated by sea-orm-codegen 0.9.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pinned_messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, unique)]
    pub original_message: i64,
    #[sea_orm(unique)]
    pub pin_message: i64,
    pub server_id: i64
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}
