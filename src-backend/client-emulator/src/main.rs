use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use env_logger::Env;
use log::debug;
use opaque_ke::{
    CipherSuite, ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialResponse, RegistrationResponse, Ristretto255,
    argon2::Argon2,
};
use rand::rngs::OsRng;
use reqwest::{
    Url,
    blocking::{Client, ClientBuilder},
    cookie::{CookieStore, Jar},
};
use rpassword::read_password;
use serde::{Deserialize, Serialize};
use sha2::Sha512;

/// rscord Client Emulator
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Server base URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    server: String,

    /// Email address for registration
    #[arg(short, long)]
    email: String,

    /// Desired handle/username
    #[arg(long)]
    handle: String,

    /// Password (if omitted, will prompt)
    #[arg(short, long)]
    password: Option<String>,

    /// Verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

struct DefaultCipherSuite;
impl CipherSuite for DefaultCipherSuite {
    type OprfCs = Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<Ristretto255, Sha512>;
    type Ksf = Argon2<'static>;
}

#[derive(Serialize)]
struct RegisterStartReq<'a> {
    client_message: &'a str,
    email: &'a str,
    handle: &'a str,
}

#[derive(Deserialize)]
struct RegisterStartResp {
    registration_id: String,
    server_message: String,
}

#[derive(Serialize)]
struct RegisterFinishReq<'a> {
    registration_id: &'a str,
    client_message: &'a str,
}

#[derive(Serialize)]
struct LoginStartReq<'a> {
    client_message: &'a str,
    email: &'a str,
}

#[derive(Deserialize)]
struct LoginStartResp {
    login_id: String,
    server_message: String,
}

#[derive(Serialize)]
struct LoginFinishReq<'a> {
    login_id: &'a str,
    client_message: &'a str,
}

#[derive(Deserialize, Debug)]
struct User {
    id: i64,
    email: String,
    handle: String,
}

#[derive(Deserialize, Debug)]
struct AuthResponse {
    token: String,
    user_id: i64,
    expires_at: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    env_logger::Builder::from_env(Env::default().default_filter_or(log_level)).init();

    let password = if let Some(p) = args.password {
        p
    } else {
        eprint!("Password: ");
        read_password().context("Failed to read password from stdin")?
    };

     run_register(&args.server, &args.email, &args.handle, password.as_bytes())
         .map_err(|e| anyhow!("Registration failed: {e:#}"))?;
    // run_login(&args.server, &args.email, password.as_bytes())
    //     .map_err(|e| anyhow!("Login failed: {e:#}"))?;

    Ok(())
}

fn run_register(server: &str, email: &str, handle: &str, password: &[u8]) -> Result<()> {
    let client = Client::new();

    let mut client_rng = OsRng;
    let client_registration_start =
        ClientRegistration::<DefaultCipherSuite>::start(&mut client_rng, password)
            .context("Failed to start client registration")?;
    let client_msg_hex = hex::encode(client_registration_start.message.serialize());
    debug!("client_message: {client_msg_hex}");

    let start_url = format!(
        "{}/api/auth/opaque/register-start",
        server.trim_end_matches('/')
    );
    let res: RegisterStartResp = client
        .post(&start_url)
        .json(&RegisterStartReq {
            client_message: &client_msg_hex,
            email,
            handle,
        })
        .send()
        .context("POST register-start request failed")?
        .error_for_status()
        .context("register-start returned error status")?
        .json()
        .context("Failed to parse register-start response JSON")?;

    let server_msg_bytes = hex::decode(&res.server_message)
        .context("Failed to hex-decode server_message from register-start")?;
    let server_response =
        RegistrationResponse::<DefaultCipherSuite>::deserialize(&server_msg_bytes)
            .context("Failed to deserialize RegistrationResponse")?;

    let mut client_rng = OsRng;
    let registration_finish_result = client_registration_start
        .state
        .finish(
            &mut client_rng,
            password,
            server_response,
            ClientRegistrationFinishParameters::default(),
        )
        .context("ClientRegistration finish failed")?;

    let upload_msg = registration_finish_result.message;
    let upload_msg_hex = hex::encode(upload_msg.serialize());
    debug!("client_message: {upload_msg_hex}");

    let finish_url = format!(
        "{}/api/auth/opaque/register-finish",
        server.trim_end_matches('/')
    );
    let finish_res = client
        .post(&finish_url)
        .json(&RegisterFinishReq {
            registration_id: &res.registration_id,
            client_message: &upload_msg_hex,
        })
        .send()
        .context("POST register-finish request failed")?;

    let status = finish_res.status();
    if status.is_success() {
        let user: User = finish_res.json()?;
        println!("{user:#?}");
    } else {
        let body = finish_res.text()?;
        eprintln!("{body}");
    }

    Ok(())
}

fn run_login(server: &str, email: &str, password: &[u8]) -> Result<()> {
    let cookie_jar = Arc::new(Jar::default());
    let client = ClientBuilder::new()
        .cookie_provider(cookie_jar.clone())
        .build()
        .context("Failed to build Reqwest client")?;

    let mut client_rng = OsRng;
    let client_login_start = ClientLogin::<DefaultCipherSuite>::start(&mut client_rng, password)
        .context("Failed to start client login")?;
    let client_msg_hex = hex::encode(client_login_start.message.serialize());
    debug!("login client_message: {client_msg_hex}");

    let start_url = format!(
        "{}/api/auth/opaque/login-start",
        server.trim_end_matches('/')
    );
    let res: LoginStartResp = client
        .post(&start_url)
        .json(&LoginStartReq {
            client_message: &client_msg_hex,
            email,
        })
        .send()
        .context("POST login-start request failed")?
        .error_for_status()
        .context("login-start returned error status")?
        .json()
        .context("Failed to parse login-start response JSON")?;

    let server_msg_bytes = hex::decode(&res.server_message)
        .context("Failed to hex-decode server_message from login-start")?;
    let server_response = CredentialResponse::<DefaultCipherSuite>::deserialize(&server_msg_bytes)
        .context("Failed to deserialize CredentialResponse")?;

    let mut client_rng = OsRng;
    let login_finish_result = client_login_start
        .state
        .finish(
            &mut client_rng,
            password,
            server_response,
            ClientLoginFinishParameters::default(),
        )
        .context("ClientLogin finish failed")?;

    let finish_msg = login_finish_result.message;
    let finish_msg_hex = hex::encode(finish_msg.serialize());
    debug!("finish client_message: {finish_msg_hex}");
    debug!("finish login_id: {}", &res.login_id);

    let finish_url = format!(
        "{}/api/auth/opaque/login-finish",
        server.trim_end_matches('/')
    );
    let finish_res = client
        .post(&finish_url)
        .json(&LoginFinishReq {
            login_id: &res.login_id,
            client_message: &finish_msg_hex,
        })
        .send()
        .context("POST login-finish request failed")?;

    let status = finish_res.status();
    if status.is_success() {
        let auth: AuthResponse = finish_res.json()?;
        println!("Auth response: {auth:#?}");

        // Print the session cookie, if any!
        let cookie_header = if let Some(c) = cookie_jar.cookies(&Url::parse(&server)?) {
            c
        } else {
            println!("(No cookies returned for this server)");
            return Ok(());
        };

        let cookie_str = if let Ok(s) = cookie_header.to_str() {
            s
        } else {
            println!("(Cookie header was not valid UTF-8)");
            return Ok(());
        };

        if let Some(cookie) = cookie_str
            .split(';')
            .find(|c| c.trim().starts_with("session_token="))
        {
            println!("Received session cookie: {cookie}");
        } else {
            println!("(No session_token cookie found in jar)");
        }
    } else {
        let body = finish_res.text()?;
        eprintln!("Login failed. Server response: {body}");
    }

    Ok(())
}
