use anyhow::{Context, Result, anyhow};
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

use crate::api::BASE_URL;

#[derive(Debug, Deserialize)]
pub struct RegisteredUser {
    id: i64,

    email: String,
    handle: String,
}

#[derive(Deserialize, Debug)]
pub struct LoggedInUser {
    id: i64,

    email: String,
    handle: String,

    settings: serde_json::Value,
    email_verified: bool,

    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,

    session: AuthResponse,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    user_id: i64,

    token: String,
    expires_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
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

#[tauri::command]
pub async fn sign_up(email: &str, handle: &str, password: &[u8]) -> Result<RegisteredUser> {
    let client = Client::new();

    let mut client_rng = OsRng;
    let client_registration_start =
        ClientRegistration::<DefaultCipherSuite>::start(&mut client_rng, password)
            .context("Failed to start client registration!")?;
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
        .context("POST register-start request failed!")?
        .error_for_status()
        .context("register-start returned error status!")?
        .json()
        .await
        .context("Failed to parse register-start response JSON!")?;

    let server_msg_bytes = hex::decode(&res.server_message)
        .context("Failed to hex-decode server_message from register-start!")?;
    let server_response =
        RegistrationResponse::<DefaultCipherSuite>::deserialize(&server_msg_bytes)
            .context("Failed to deserialize RegistrationResponse!")?;

    let mut client_rng = OsRng;
    let registration_finish_result = client_registration_start
        .state
        .finish(
            &mut client_rng,
            password,
            server_response,
            ClientRegistrationFinishParameters::default(),
        )
        .context("ClientRegistration finish failed!")?;

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
        .context("POST register-finish request failed")?;

    let status = finish_res.status();
    if status.is_success() {
        let user: RegisteredUser = finish_res.json().await?;

        Ok(user)
    } else {
        let body = finish_res.text().await?;

        Err(anyhow!(body))
    }
}

#[tauri::command]
pub async fn log_in(email: &str, password: &[u8]) -> Result<LoggedInUser> {
    let cookie_jar = Arc::new(Jar::default());
    let client = ClientBuilder::new()
        .cookie_provider(cookie_jar.clone())
        .build()
        .context("Failed to build Reqwest client!")?;

    let mut client_rng = OsRng;
    let client_login_start = ClientLogin::<DefaultCipherSuite>::start(&mut client_rng, password)
        .context("Failed to start client login!")?;
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
        .context("POST login-start request failed")?
        .error_for_status()
        .context("login-start returned error status")?
        .json()
        .await
        .context("Failed to parse login-start response JSON")?;

    let server_msg_bytes = hex::decode(&res.server_message)
        .context("Failed to hex-decode server_message from login-start!")?;
    let server_response = CredentialResponse::<DefaultCipherSuite>::deserialize(&server_msg_bytes)
        .context("Failed to deserialize CredentialResponse!")?;

    let mut client_rng = OsRng;
    let login_finish_result = client_login_start
        .state
        .finish(
            &mut client_rng,
            password,
            server_response,
            ClientLoginFinishParameters::default(),
        )
        .context("ClientLogin finish failed!")?;

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
        .context("POST login-finish request failed")?;

    let status = finish_res.status();
    if !status.is_success() {
        let body = finish_res.text().await?;

        return Err(anyhow!("Login failed, server response: {body}"));
    }

    let auth: AuthResponse = finish_res.json().await?;

    // Validate cookies.
    let cookie_header = if let Some(c) = cookie_jar.cookies(&Url::parse(&BASE_URL)?) {
        c
    } else {
        return Err(anyhow!("No cookies returned from server!"));
    };

    let cookie_str = if let Ok(s) = cookie_header.to_str() {
        s
    } else {
        return Err(anyhow!("Cookie header was not valid UTF-8!"));
    };

    if let Some(cookie) = cookie_str
        .split(';')
        .find(|c| c.trim().starts_with("session_token="))
    {
        info!("Received session cookie: {cookie}");
    } else {
        return Err(anyhow!("No session_token cookie found in jar!"));
    }

    let user_endpoint = format!("{BASE_URL}/api/users/{}", auth.user_id);
    let user_res = client.get(&user_endpoint).send().await?;
    let status = user_res.status();
    if !status.is_success() {
        let body = user_res.text().await?;

        return Err(anyhow!("Failed to get user details: {body}"));
    }

    let user = user_res.json().await?;

    Ok(user)
}
