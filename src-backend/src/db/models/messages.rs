use crate::db::schema::guild_messages;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = guild_messages)]
pub struct GuildMessage {
    pub id: i64,
    pub author_id: i64,
    pub reply_to_id: Option<i64>,
    pub channel_id: i64,
    pub contents: Option<String>,
    pub edited_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = guild_messages)]
pub struct NewGuildMessage {
    pub reply_to_id: Option<i64>,
    pub contents: Option<String>,
}