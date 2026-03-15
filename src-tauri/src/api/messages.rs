use crate::api::token::get_token;
use crate::api::BASE_URL;
use crate::AppClientState;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuildMessage {
    pub id: i64,
    pub author_id: i64,
    pub reply_to_id: Option<i64>,
    pub channel_id: i64,
    pub contents: Option<String>,
    pub edited_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

/// Sends a new message to the specified channel.
///
/// # Errors
///
/// Returns an error string if:
/// * The network request fails (e.g., connection issues).
/// * The API returns a non-success status code (e.g., 400 Bad Request, 401 Unauthorized).
/// * The API response fails to deserialize into a `GuildMessage`.
#[tauri::command(async)]
pub async fn send_message(
    state: tauri::State<'_, AppClientState>,
    channel_id: i64,
    content: String,
) -> Result<GuildMessage, String> {
    let token = get_token(&state);
    let url = format!("{BASE_URL}/api/channels/{channel_id}/messages");

    let res = state
        .client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "contents": content }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("API error: {e}"))?
        .json::<GuildMessage>()
        .await
        .map_err(|e| format!("Deserialization error: {e}"))?;

    Ok(res)
}

/// Retrieves a list of messages for the specified channel.
///
/// # Errors
///
/// Returns an error string if:
/// * The network request fails (e.g., DNS issues, connection timeout).
/// * The API returns a non-success status code (e.g., 401 Unauthorized, 404 Not Found).
/// * The response body fails to deserialize into a `Vec<GuildMessage>`.
#[tauri::command(async)]
pub async fn get_messages(
    state: tauri::State<'_, AppClientState>,
    channel_id: i64,
) -> Result<Vec<GuildMessage>, String> {
    let url = format!("{BASE_URL}/api/channels/{channel_id}/messages");
    let token = get_token(&state);

    let res = state
        .client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("API returned error: {e}"))?
        .json::<Vec<GuildMessage>>()
        .await
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    Ok(res)
}
