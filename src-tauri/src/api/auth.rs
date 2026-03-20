use chrono::NaiveDateTime;
use log::info;
use opaque_ke::{
    CipherSuite, ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialResponse, RegistrationResponse, Ristretto255,
    argon2::Argon2, rand::rngs::OsRng,
};

use reqwest::{
    Client, ClientBuilder, Url,
    cookie::{CookieStore, Jar},
};

use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::sync::Arc;

use crate::AppClientState;
use crate::api::BASE_URL;
use crate::api::token::save_token;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredUser {
    id: i64,

    email: String,
    handle: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoggedInUser {
    user: User,
    auth: AuthResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    user_id: i64,

    token: String,
    // Deserialize the field named "expires_at" from the backend server,
    // but serialize it as "expires" to the frontend.
    #[serde(rename(deserialize = "expires_at", serialize = "expires"))]
    expires: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,

    pub handle: String,
    pub settings: serde_json::Value,
    pub email_verified: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

struct DefaultCipherSuite;
impl CipherSuite for DefaultCipherSuite {
    type OprfCs = Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<Ristretto255, Sha512>;
    type Ksf = Argon2<'static>;
}

#[derive(Debug, Serialize)]
struct RegisterStartReq<'a> {
    client_message: &'a str,
    email: &'a str,
    handle: &'a str,
}

#[derive(Debug, Deserialize)]
struct RegisterStartResp {
    registration_id: String,
    server_message: String,
}

#[derive(Debug, Serialize)]
struct RegisterFinishReq<'a> {
    registration_id: &'a str,
    client_message: &'a str,
}

#[derive(Debug, Serialize)]
struct LoginStartReq<'a> {
    client_message: &'a str,
    email: &'a str,
}

#[derive(Debug, Deserialize)]
struct LoginStartResp {
    login_id: String,
    server_message: String,
}

#[derive(Debug, Serialize)]
struct LoginFinishReq<'a> {
    login_id: &'a str,
    client_message: &'a str,
}

#[tauri::command(async)]
pub async fn sign_up(email: &str, handle: &str, password: &str) -> Result<RegisteredUser, String> {
    let password = password.as_bytes();
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut client_rng = OsRng;
    let client_registration_start =
        ClientRegistration::<DefaultCipherSuite>::start(&mut client_rng, password)
            .map_err(|e| e.to_string())?;
    let client_msg_hex = hex::encode(client_registration_start.message.serialize());
    let start_url = format!("{BASE_URL}/api/auth/opaque/register-start");
    let res: RegisterStartResp = client
        .post(&start_url)
        .json(&RegisterStartReq {
            client_message: &client_msg_hex,
            email,
            handle,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let server_msg_bytes = hex::decode(&res.server_message).map_err(|e| e.to_string())?;
    let server_response =
        RegistrationResponse::<DefaultCipherSuite>::deserialize(&server_msg_bytes)
            .map_err(|e| e.to_string())?;

    let mut client_rng = OsRng;
    let registration_finish_result = client_registration_start
        .state
        .finish(
            &mut client_rng,
            password,
            server_response,
            ClientRegistrationFinishParameters::default(),
        )
        .map_err(|e| e.to_string())?;

    let upload_msg = registration_finish_result.message;
    let upload_msg_hex = hex::encode(upload_msg.serialize());
    let finish_url = format!("{BASE_URL}/api/auth/opaque/register-finish");
    let finish_res = client
        .post(&finish_url)
        .json(&RegisterFinishReq {
            registration_id: &res.registration_id,
            client_message: &upload_msg_hex,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = finish_res.status();
    if status.is_success() {
        let user: RegisteredUser = finish_res.json().await.map_err(|e| e.to_string())?;

        Ok(user)
    } else {
        let body = finish_res.text().await.map_err(|e| e.to_string())?;

        Err(body)
    }
}

#[tauri::command(async)]
pub async fn log_in(
    email: &str,
    password: &str,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppClientState>,
) -> Result<LoggedInUser, String> {
    let password = password.as_bytes();
    let cookie_jar = Arc::new(Jar::default());
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .cookie_provider(cookie_jar.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let mut client_rng = OsRng;
    let client_login_start = ClientLogin::<DefaultCipherSuite>::start(&mut client_rng, password)
        .map_err(|e| e.to_string())?;
    let client_msg_hex = hex::encode(client_login_start.message.serialize());

    let start_url = format!("{BASE_URL}/api/auth/opaque/login-start");
    let res: LoginStartResp = client
        .post(&start_url)
        .json(&LoginStartReq {
            client_message: &client_msg_hex,
            email,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let server_msg_bytes = hex::decode(&res.server_message).map_err(|e| e.to_string())?;
    let server_response = CredentialResponse::<DefaultCipherSuite>::deserialize(&server_msg_bytes)
        .map_err(|e| e.to_string())?;

    let mut client_rng = OsRng;
    let login_finish_result = client_login_start
        .state
        .finish(
            &mut client_rng,
            password,
            server_response,
            ClientLoginFinishParameters::default(),
        )
        .map_err(|e| e.to_string())?;

    let finish_msg = login_finish_result.message;
    let finish_msg_hex = hex::encode(finish_msg.serialize());
    let finish_url = format!("{BASE_URL}/api/auth/opaque/login-finish");
    let finish_res = client
        .post(&finish_url)
        .json(&LoginFinishReq {
            login_id: &res.login_id,
            client_message: &finish_msg_hex,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = finish_res.status();
    if !status.is_success() {
        let body = finish_res.text().await.map_err(|e| e.to_string())?;

        return Err(format!("Login failed, server response: {body}"));
    }

    let auth: AuthResponse = finish_res.json().await.map_err(|e| e.to_string())?;

    // Validate cookies.
    let cookie_header =
        if let Some(c) = cookie_jar.cookies(&Url::parse(&BASE_URL).map_err(|e| e.to_string())?) {
            c
        } else {
            return Err("No cookies returned from server!".to_string());
        };

    let cookie_str = if let Ok(s) = cookie_header.to_str() {
        s
    } else {
        return Err("Cookie header was not valid UTF-8!".to_string());
    };

    if let Some(cookie) = cookie_str
        .split(';')
        .find(|c| c.trim().starts_with("session_token="))
    {
        info!("Received session cookie: {cookie}");
    } else {
        return Err("No session_token cookie found in jar!".to_string());
    }

    let user_endpoint = format!("{BASE_URL}/api/users/{}", auth.user_id);
    let user_res = client
        .get(&user_endpoint)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = user_res.status();
    if !status.is_success() {
        let body = user_res.text().await.map_err(|e| e.to_string())?;

        return Err(format!("Failed to get user details: {body}"));
    }

    let user = user_res.json().await.map_err(|e| e.to_string())?;

    // Persist the token to the on-disk store and update in-memory state so
    // subsequent Tauri commands (e.g. init_websocket) can access it without
    // needing the frontend to round-trip it back via set_token.
    save_token(&app, &state, &auth.token);

    Ok(LoggedInUser { user, auth })
}
