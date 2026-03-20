pub mod api;

use tauri::Manager;

use crate::api::{
    auth::{log_in, sign_up},
    guilds::{
        create_channel, create_guild, delete_guild, get_guild_channels, get_guild_members,
        join_guild, leave_guild, list_my_guilds,
    },
    messages::{get_messages, send_message},
    push_token::add_push_token,
    token::{remove_token, restore_token},
    ws_client::init_websocket,
};
use std::sync::Mutex;

// TODO: update with real login logic and not use of session token.
pub struct AppClientState {
    pub client: reqwest::Client,
    pub token: Mutex<String>,
    /// Handle to the active WebSocket background task. Used to cancel a
    /// previous connection before starting a new one, ensuring at most one
    /// live connection at any time.
    pub ws_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

/// Initializes and runs the Tauri application.
///
/// # Panics
///
/// Panics if the application fails to start (e.g., if the webview or event loop initialization fails).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notifications::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppClientState {
            client: reqwest::Client::new(),
            token: String::new().into(),
            ws_task: Mutex::new(None),
        })
        .setup(|app| {
            // Restore a previously persisted session token so the backend
            // state is ready before any command is invoked, without requiring
            // the frontend to round-trip the token back via set_token.
            let state = app.state::<AppClientState>();
            restore_token(app.handle(), &state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_push_token,
            sign_up,
            log_in,
            send_message,
            list_my_guilds,
            create_guild,
            join_guild,
            create_channel,
            leave_guild,
            delete_guild,
            get_guild_members,
            get_guild_channels,
            get_messages,
            remove_token,
            init_websocket,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
