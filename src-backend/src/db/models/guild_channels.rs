use crate::db::schema::guild_channels;
use crate::db::models::channels::ChannelType;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = crate::db::schema::guild_channels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildChannel {
    pub id: i64,
    pub guild_id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: ChannelType,
    pub topic: Option<String>,
    pub position: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = guild_channels)]
pub struct NewGuildChannel {
    #[serde(skip_deserializing)]
    pub guild_id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: ChannelType,
    pub topic: Option<String>,
    pub position: Option<i32>,
}