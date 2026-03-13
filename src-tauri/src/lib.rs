pub mod api;
use crate::api::{
    auth::{log_in, sign_up},
    push_token::add_push_token,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notifications::init())
        .invoke_handler(tauri::generate_handler![
            add_push_token,
            sign_up,
            log_in
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
