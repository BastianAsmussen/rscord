use crate::api::BASE_URL;
use serde_json::json;

#[tauri::command]
//TODO: add proper authentication once login has been merged into master
pub async fn add_push_token(user_id: i32, token: &str) -> Result<(), String> {
    let request_url = format!("{base_url}/api/push-token", base_url = BASE_URL);
    let body = json!({
        "user_id": user_id,
        "token": token
    });

    let response = reqwest::Client::new()
        .post(request_url)
        .json(&body)
        .send().await;
    println!("{:?}", response);

    match response {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("{}", e))
    }
}