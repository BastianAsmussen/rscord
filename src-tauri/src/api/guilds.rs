use crate::AppClientState;
use crate::api::BASE_URL;
use crate::api::token::get_token;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Channel {
    pub id: i64,
    pub guild_id: i64,

    #[serde(rename = "type")]
    pub channel_type: String,

    pub name: String,
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateChannel {
    pub channel_type: String,
    pub name: String,
    pub topic: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuildSummary {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleSummary {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuildMemberWithRoles {
    pub user_id: i64,
    pub user_handle: String,
    pub roles: Vec<RoleSummary>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGuild {
    pub name: String,
}

/// Fetch all channels for a specific guild.
///
/// # Errors
/// - Returns a string error if the network request fails, the API returns a non-success status,
///   or the response body cannot be deserialized into a list of `Channel`.
#[tauri::command(async)]
pub async fn get_guild_channels(
    id: i64,
    state: tauri::State<'_, AppClientState>,
) -> Result<Vec<Channel>, String> {
    let url = format!("{BASE_URL}/api/guilds/{id}/channels");
    let token = get_token(&state);

    state
        .client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Vec<Channel>>()
        .await
        .map_err(|e| e.to_string())
}

/// List all guilds the current user is a member of.
///
/// # Errors
/// - Returns a string error if the network request fails, the API returns a non-success status,
///   or the response body cannot be deserialized into a list of `GuildSummary`.
#[tauri::command(async)]
pub async fn list_my_guilds(
    state: tauri::State<'_, AppClientState>,
) -> Result<Vec<GuildSummary>, String> {
    let url = format!("{BASE_URL}/api/guilds");
    let token = get_token(&state);

    state
        .client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Vec<GuildSummary>>()
        .await
        .map_err(|e| e.to_string())
}

/// Fetch all members and their roles for a specific guild.
///
/// # Errors
/// - Returns a string error if the network request fails, the API returns a non-success status,
///   or the response body cannot be deserialized into a list of `GuildMemberWithRoles`.
#[tauri::command(async)]
pub async fn get_guild_members(
    id: i64,
    state: tauri::State<'_, AppClientState>,
) -> Result<Vec<GuildMemberWithRoles>, String> {
    let url = format!("{BASE_URL}/api/guilds/{id}/members");
    let token = get_token(&state);

    state
        .client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Vec<GuildMemberWithRoles>>()
        .await
        .map_err(|e| e.to_string())
}

/// Creates a new guild and sets the current user as the owner.
///
/// # Errors
///
/// Returns an error string if:
/// * No authentication token is found in the application state.
/// * The network request fails or the API is unreachable.
/// * The API returns a non-success status code (e.g., 401 Unauthorized).
/// * The response body fails to deserialize into a `GuildSummary`.
#[tauri::command(async)]
pub async fn create_guild(
    name: String,
    state: tauri::State<'_, AppClientState>,
) -> Result<GuildSummary, String> {
    let url = format!("{BASE_URL}/api/guilds");
    let token = get_token(&state);

    if token.is_empty() {
        return Err("No session token found".to_string());
    }

    state
        .client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&CreateGuild { name })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<GuildSummary>()
        .await
        .map_err(|e| e.to_string())
}

/// Joins a guild using its unique ID.
///
/// # Errors
///
/// Returns an error string if:
/// * The network request fails (e.g., timeout or connection issues).
/// * The API returns a non-success status code (e.g., 404 Not Found, 422 Already a Member).
/// * The response body fails to deserialize into a `GuildSummary`.
#[tauri::command(async)]
pub async fn join_guild(
    id: i64,
    state: tauri::State<'_, AppClientState>,
) -> Result<GuildSummary, String> {
    let url = format!("{BASE_URL}/api/guilds/{id}/join");
    let token = get_token(&state);

    state
        .client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("API error: {e}"))?
        .json::<GuildSummary>()
        .await
        .map_err(|e| format!("Deserialization failed: {e}"))
}

/// Creates a new channel within a guild.
///
/// # Errors
///
/// Returns an error string if:
/// * The authentication token is missing or invalid.
/// * The network request fails or the API returns a 422 (e.g., missing fields like position).
/// * The backend response fails to deserialize into a `Channel`.
#[tauri::command(async)]
pub async fn create_channel(
    guild_id: i64,
    name: String,
    channel_type: String,
    topic: String,
    state: tauri::State<'_, AppClientState>,
) -> Result<Channel, String> {
    let url = format!("{BASE_URL}/api/guilds/{guild_id}/channels");
    let token = get_token(&state);

    let payload = serde_json::json!({
        "type": channel_type,
        "name": name,
        "position": 0,
        "properties": {
            "topic": topic
        }
    });

    state
        .client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?
        .error_for_status()
        .map_err(|e| format!("API error: {e}"))?
        .json::<Channel>()
        .await
        .map_err(|e| format!("Deserialization error: {e}"))
}

/// Leaves a guild.
/// # Errors
/// Returns an error if the network request fails or the user is the owner (owners must delete).
#[tauri::command(async)]
pub async fn leave_guild(id: i64, state: tauri::State<'_, AppClientState>) -> Result<(), String> {
    let url = format!("{BASE_URL}/api/guilds/{id}/leave");
    let token = get_token(&state);

    state
        .client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes a guild (Owner only).
/// # Errors
/// Returns an error if the network request fails or user is not the owner.
#[tauri::command(async)]
pub async fn delete_guild(id: i64, state: tauri::State<'_, AppClientState>) -> Result<(), String> {
    let url = format!("{BASE_URL}/api/guilds/{id}");
    let token = get_token(&state);

    state
        .client
        .delete(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    Ok(())
}
