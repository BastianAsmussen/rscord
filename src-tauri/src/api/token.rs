use crate::AppClientState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SessionWrapper {
    token: String,
}

/// Sets the session token.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
#[tauri::command]
pub fn set_token(token_json: String, state: tauri::State<'_, AppClientState>) {
    let mut token_lock = state.token.lock().unwrap();
    *token_lock = token_json;
}

/// Removes the session token.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
#[tauri::command]
pub fn remove_token(state: tauri::State<'_, AppClientState>) {
    let mut token_lock = state.token.lock().unwrap();
    *token_lock = String::new();
}

/// Retrieves the current session token from the application state.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
#[must_use]
pub fn get_token(state: &tauri::State<'_, AppClientState>) -> String {
    let lock = state.token.lock().unwrap();

    serde_json::from_str::<SessionWrapper>(&lock)
        .map(|wrapper| wrapper.token)
        .unwrap_or_default()
}