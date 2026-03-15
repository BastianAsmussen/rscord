use crate::AppClientState;

// TODO: This is just a temp. file for testing with tokens, needs to be using auth in the future.

/// Sets the session token.
///
/// # Panics
///
/// Panics if the token storage is locked by a thread that crashed.
#[tauri::command]
pub fn set_token(token: String, state: tauri::State<'_, AppClientState>) {
    let mut token_lock = state.token.lock().unwrap();
    *token_lock = token;
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
pub fn get_token(state: &tauri::State<'_, AppClientState>) -> String {
    state.token.lock().unwrap().clone()
}
