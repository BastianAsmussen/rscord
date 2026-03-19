use crate::AppClientState;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "session.json";
const TOKEN_KEY: &str = "token";

/// Persists `token` to the on-disk store and updates the in-memory state.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
pub fn save_token(app: &AppHandle, state: &tauri::State<'_, AppClientState>, token: &str) {
    *state.token.lock().unwrap() = token.to_owned();
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(TOKEN_KEY, token);
    }
}

/// Removes the session token from both the in-memory state and the on-disk store.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
#[tauri::command]
pub fn remove_token(app: AppHandle, state: tauri::State<'_, AppClientState>) {
    *state.token.lock().unwrap() = String::new();
    if let Ok(store) = app.store(STORE_FILE) {
        store.delete(TOKEN_KEY);
    }
}

/// Retrieves the current session token from the in-memory application state.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
#[must_use]
pub fn get_token(state: &tauri::State<'_, AppClientState>) -> String {
    state.token.lock().unwrap().clone()
}

/// Loads the persisted token from the on-disk store into the in-memory state.
/// Called once during app setup so subsequent commands find the token ready.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
pub fn restore_token(app: &AppHandle, state: &tauri::State<'_, AppClientState>) {
    if let Ok(store) = app.store(STORE_FILE) {
        if let Some(token) = store
            .get(TOKEN_KEY)
            .and_then(|v| v.as_str().map(String::from))
        {
            *state.token.lock().unwrap() = token;
        }
    }
}
