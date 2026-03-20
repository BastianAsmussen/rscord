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

/// A guild message enriched with the author's display name, used as the API
/// response type and the WebSocket broadcast payload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GuildMessageResponse {
    pub id: i64,
    pub author_id: i64,
    pub author_name: String,
    pub reply_to_id: Option<i64>,
    pub channel_id: i64,
    pub contents: Option<String>,
    pub edited_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

impl GuildMessageResponse {
    #[must_use]
    pub fn new(msg: GuildMessage, author_name: String) -> Self {
        Self {
            id: msg.id,
            author_id: msg.author_id,
            author_name,
            reply_to_id: msg.reply_to_id,
            channel_id: msg.channel_id,
            contents: msg.contents,
            edited_at: msg.edited_at,
            created_at: msg.created_at,
        }
    }
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = guild_messages)]
pub struct NewGuildMessage {
    pub reply_to_id: Option<i64>,
    pub contents: Option<String>,
}
