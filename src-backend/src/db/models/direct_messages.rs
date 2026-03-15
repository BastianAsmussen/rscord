use crate::db::schema::direct_messages;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = direct_messages)]
pub struct DirectMessage {
    pub id: i64,
    pub author_id: i64,
    pub reply_to_id: Option<i64>,
    pub channel_id: i64,
    /// AES-256-GCM encrypted message content.
    #[schema(value_type = Vec<u8>)]
    pub ciphertext: Vec<u8>,
    /// AES-256-GCM nonce (12 bytes).
    #[schema(value_type = Vec<u8>)]
    pub nonce: Vec<u8>,
    /// Client-side identifier for the ratchet key used to encrypt this message.
    pub ratchet_key_id: i64,
    pub created_at: NaiveDateTime,
}

/// Payload for sending a new encrypted direct message.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewDirectMessage {
    pub reply_to_id: Option<i64>,
    /// AES-256-GCM encrypted message content (hex-encoded).
    pub ciphertext: String,
    /// AES-256-GCM nonce (hex-encoded, 12 bytes / 24 hex chars).
    pub nonce: String,
    /// Client-side identifier for the ratchet key used to encrypt this message.
    pub ratchet_key_id: i64,
}
