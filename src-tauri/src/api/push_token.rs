use crate::AppClientState;
use crate::api::BASE_URL;
use crate::api::token::get_token;
use serde_json::json;

#[tauri::command]
//TODO: add proper authentication once login has been merged into master
pub async fn add_push_token(
    state: tauri::State<'_, AppClientState>,
    user_id: i32,
    token: &str,
) -> Result<(), String> {
    let request_url = format!("{base_url}/api/push-token", base_url = BASE_URL);
    let token = get_token(&state);
    let body = json!({
        "user_id": user_id,
        "token": token
    });

    let response = reqwest::Client::new()
        .post(request_url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await;

    if let Err(e) = response {
        return Err(e.to_string());
    }

    println!("{:?}", response);

    Ok(())
}
