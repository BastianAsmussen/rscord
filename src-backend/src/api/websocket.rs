use crate::api::auth_extractor::AuthUser;
use crate::api::opaque::AppState;
use crate::db::schema::{channels, guild_members};
use axum::extract::ws::Utf8Bytes;
use axum::{extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
}, response::IntoResponse, Router};
use diesel::prelude::*;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use axum::routing::get;
use tokio::sync::broadcast::error::RecvError;

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

async fn handle_socket(socket: WebSocket, state: AppState, user_id: i64) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.tx.subscribe();

    let conn = match state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get DB pool: {:?}", e);
            return;
        }
    };

    let allowed_guild_channels: HashSet<i64> = conn
        .interact(move |conn| {
            channels::table
                .inner_join(
                    guild_members::table
                        .on(channels::guild_id.eq(guild_members::guild_id.nullable())),
                )
                .filter(guild_members::user_id.eq(user_id))
                .select(channels::id)
                .load::<i64>(conn)
                .unwrap_or_default()
                .into_iter()
                .collect()
        })
        .await
        .unwrap_or_default();

    // Broadcast -> Client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if !allowed_guild_channels.contains(&msg.channel_id) {
                        continue;
                    }

                    let Ok(json) = serde_json::to_string(&msg) else {
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
                Err(RecvError::Lagged(n)) => {
                    tracing::debug!("User {user_id} lagged by {n} messages");
                }
                Err(RecvError::Closed) => break,
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
