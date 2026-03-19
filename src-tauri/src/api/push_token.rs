use crate::api::BASE_URL;
use serde_json::json;
use crate::api::token::get_token;
use crate::AppClientState;

#[tauri::command]
pub async fn add_push_token(
    state: tauri::State<'_, AppClientState>,
    token: &str
    ) -> Result<(), String> {
    let request_url = format!("{BASE_URL}/api/push-token/{token}");
    let token = get_token(&state);

    let response = reqwest::Client::new()
        .post(request_url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;

    if let Err(e) = response {
        return Err(e.to_string());
    }

    println!("{:?}", response);

    Ok(())
}
