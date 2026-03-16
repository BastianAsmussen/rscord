use crate::AppClientState;
use crate::api::WS_URL;
use crate::api::token::get_token;
use futures_util::StreamExt;
use tauri::http::Request;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Initiates a WebSocket connection and begins listening for incoming messages.
///
/// # Errors
///
/// Returns an error string if:
/// - The HTTP request builder fails to construct the handshake request.
/// - The WebSocket handshake fails (e.g., server unreachable, rejected upgrade).
#[tauri::command]
pub async fn start_ws_client(app: AppHandle, token: String) -> Result<(), String> {
    let url = format!("{WS_URL}/ws");

    let request = Request::builder()
        .method("GET")
        .uri(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Host", "127.0.0.1:8080")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key())
        .body(())
        .map_err(|e| e.to_string())?;

    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| format!("WebSocket connection failed: {e}"))?;

    let (_, mut read) = ws_stream.split();

    tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            let text_str: String = text.to_string();

            println!("WebSocket received message: {text_str}");

            if let Err(e) = app.emit("new-message", &text_str) {
                eprintln!("Failed to emit event: {e}");
            }
        }
    });

    Ok(())
}

/// Initializes the WebSocket process by retrieving the current authentication token
/// and spawning the client connection task.
///
/// # Errors
///
/// Returns an error string if:
/// - No authentication token is found in the application state.
#[tauri::command]
pub async fn init_websocket(
    app: AppHandle,
    state: tauri::State<'_, AppClientState>,
) -> Result<(), String> {
    let token = get_token(&state);

    if token.is_empty() {
        return Err("No token found".into());
    }

    let app_handle = app;

    tauri::async_runtime::spawn(async move {
        if let Err(e) = start_ws_client(app_handle, token).await {
            eprintln!("WebSocket connection error: {e}");
        }
    });

    Ok(())
}
