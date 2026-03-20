use crate::api::auth_extractor::AuthUser;
use crate::api::opaque::AppState;
use crate::db::schema::{channels, channels_members, guild_members};
use axum::extract::ws::Utf8Bytes;
use axum::routing::get;
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use diesel::prelude::*;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::collections::HashSet;

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    auth: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, auth.session.user_id))
}

/// Wrapper so the client can distinguish guild messages from encrypted DMs.
#[derive(Serialize)]
#[serde(tag = "type")]
enum WsEvent {
    /// A plaintext guild message.
    #[serde(rename = "guild_message")]
    GuildMessage(crate::db::models::guild_messages::GuildMessageResponse),
    /// An encrypted direct message (ciphertext + nonce + ratchet key id).
    #[serde(rename = "direct_message")]
    DirectMessage(crate::db::models::direct_messages::DirectMessage),
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: i64) {
    let (mut sender, mut receiver) = socket.split();

    let mut guild_rx = state.tx.subscribe();
    let mut dm_rx = state.dm_tx.subscribe();

    let conn = match state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get DB pool: {:?}", e);
            return;
        }
    };

    let (allowed_guild_channels, allowed_dm_channels) = conn
        .interact(move |conn| {
            let guild_channels: HashSet<i64> = channels::table
                .inner_join(
                    guild_members::table
                        .on(channels::guild_id.eq(guild_members::guild_id.nullable())),
                )
                .filter(guild_members::user_id.eq(user_id))
                .select(channels::id)
                .load::<i64>(conn)
                .unwrap_or_default()
                .into_iter()
                .collect();

            let dm_channels: HashSet<i64> = channels_members::table
                .filter(channels_members::user_id.eq(user_id))
                .select(channels_members::channel_id)
                .load::<i64>(conn)
                .unwrap_or_default()
                .into_iter()
                .collect();

            (guild_channels, dm_channels)
        })
        .await
        .unwrap_or_default();

    // Broadcast -> Client (guild messages + encrypted DMs)
    let mut send_task = tokio::spawn(async move {
        loop {
            let event = tokio::select! {
                result = guild_rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if !allowed_guild_channels.contains(&msg.channel_id) {
                                continue;
                            }
                            WsEvent::GuildMessage(msg)
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::debug!("User {user_id} lagged by {n} guild messages");
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                result = dm_rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if !allowed_dm_channels.contains(&msg.channel_id) {
                                continue;
                            }
                            WsEvent::DirectMessage(msg)
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::debug!("User {user_id} lagged by {n} DM messages");
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            };

            let Ok(json) = serde_json::to_string(&event) else {
                continue;
            };

            if sender
                .send(Message::Text(Utf8Bytes::from(json)))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
