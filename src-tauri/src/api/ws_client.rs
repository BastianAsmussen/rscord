use crate::AppClientState;
use crate::api::WS_URL;
use futures_util::StreamExt;
use serde::Deserialize;
use tauri::http::Request;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Typed events sent by the server over the WebSocket connection.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerEvent {
    GuildMessage(serde_json::Value),
    DirectMessage(serde_json::Value),
}

/// Opens a WebSocket connection and emits typed Tauri events for each
/// incoming server message until the connection closes.
async fn run_ws_connection(app: AppHandle, token: String) {
    let url = format!("{WS_URL}/ws");

    let request = match Request::builder()
        .method("GET")
        .uri(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key())
        // The Host header is intentionally omitted here; tokio-tungstenite
        // derives it from the request URI, which is correct for both dev
        // (localhost:8080) and production (rscord.asmussen.tech).
        .body(())
    {
        Ok(r) => r,
        Err(e) => {
            log::error!("Failed to build WebSocket request: {e}");
            return;
        }
    };

    let (ws_stream, _) = match connect_async(request).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("WebSocket connection failed: {e}");
            return;
        }
    };

    let (_, mut read) = ws_stream.split();

    while let Some(Ok(Message::Text(text))) = read.next().await {
        match serde_json::from_str::<ServerEvent>(&text) {
            Ok(ServerEvent::GuildMessage(payload)) => {
                if let Err(e) = app.emit("guild-message", payload) {
                    log::error!("Failed to emit guild-message event: {e}");
                }
            }
            Ok(ServerEvent::DirectMessage(payload)) => {
                if let Err(e) = app.emit("dm-message", payload) {
                    log::error!("Failed to emit dm-message event: {e}");
                }
            }
            Err(e) => {
                log::warn!("Failed to parse WebSocket event: {e}");
            }
        }
    }
}

/// Initializes the WebSocket connection, cancelling any previously active one.
///
/// `token` is the session token supplied by the frontend (read from its
/// persistent storage). It is saved into the shared [`AppClientState`] and the
/// on-disk store so that all subsequent Tauri commands have it available
/// without requiring a separate `set_token` round-trip.
///
/// This ensures at most one live connection exists at any time, preventing
/// duplicate message delivery when the command is invoked multiple times
/// (e.g., on component remount).
///
/// # Errors
///
/// Returns an error string if the supplied token is empty.
#[tauri::command]
pub async fn init_websocket(
    token: String,
    app: AppHandle,
    state: tauri::State<'_, AppClientState>,
) -> Result<(), String> {
    if token.is_empty() {
        return Err("No token found".into());
    }

    // Persist the token so every subsequent command can read it from state,
    // and so app restarts can restore it from the on-disk store.
    crate::api::token::save_token(&app, &state, &token);

    let task = tauri::async_runtime::spawn(async move {
        run_ws_connection(app, token).await;
    });

    // Cancel any previously active connection, then store the new task handle.
    let old_task = state
        .ws_task
        .lock()
        .expect("ws_task mutex poisoned")
        .replace(task);

    if let Some(old) = old_task {
        old.abort();
    }

    Ok(())
}
