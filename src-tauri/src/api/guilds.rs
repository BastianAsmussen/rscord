use crate::api::token::get_token;
use crate::api::BASE_URL;
use crate::AppClientState;
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
