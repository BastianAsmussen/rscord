use crate::db::schema::channels;
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, DbEnum, ToSchema)]
#[ExistingTypePath = "crate::db::schema::sql_types::ChannelType"]
pub enum ChannelType {
    Text,
    Voice,
    Dm,
    GroupDm,
}

#[derive(Debug, Serialize, Queryable, Selectable, Identifiable, ToSchema)]
#[diesel(table_name = channels)]
pub struct Channel {
    pub id: i64,
    pub guild_id: Option<i64>,
    #[serde(rename = "type")]
    pub type_: ChannelType,
    pub name: Option<String>,
    pub position: i32,
    pub properties: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = channels)]
pub struct NewChannel {
    pub guild_id: Option<i64>,
    #[serde(rename = "type")]
    pub type_: ChannelType,
    pub name: Option<String>,
    pub position: i32,
    pub properties: serde_json::Value,
}

#[derive(Debug, AsChangeset, Deserialize, Serialize, ToSchema)]
#[diesel(table_name = channels)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub position: Option<i32>,
    pub properties: Option<serde_json::Value>,
}
