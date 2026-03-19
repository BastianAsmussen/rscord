use argon2::password_hash::rand_core::OsRng;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
    associations::HasTable,
};
use opaque_ke::{
    CredentialFinalization, CredentialRequest, RegistrationRequest, RegistrationUpload,
    ServerLogin, ServerLoginParameters, ServerRegistration,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{auth_extractor::AuthUser, errors::ApiError, opaque::DefaultCipherSuite};
use crate::{
    api::{errors::ErrorBody, opaque::AppState},
    db::{
        models::{
            sessions::NewSession,
            users::{NewUser, User},
        },
        schema::{
            displayed_users as displayed_users_schema, sessions as sessions_schema,
            users as users_schema,
        },
    },
};

const SESSION_DURATION_DAYS: i64 = 30;

#[derive(Debug, Deserialize, ToSchema)]
pub struct OpaqueRegisterStartRequest {
    pub email: String,
    pub handle: String,
    /// `RegistrationRequest` serialized by the client, hex-encoded.
    pub client_message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpaqueRegisterStartResponse {
    /// Opaque token identifying this handshake; send back in finish.
    pub registration_id: String,
    /// `RegistrationResponse` for the client, hex-encoded.
    pub server_message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OpaqueRegisterFinishRequest {
    pub registration_id: String,
    /// `RegistrationUpload` serialized by the client, hex-encoded.
    pub client_message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OpaqueLoginStartRequest {
    pub email: String,
    /// `CredentialRequest` serialized by the client, hex-encoded.
    pub client_message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpaqueLoginStartResponse {
    pub login_id: String,
    /// `CredentialResponse` for the client, hex-encoded.
    pub server_message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OpaqueLoginFinishRequest {
    pub login_id: String,
    /// `CredentialFinalization` serialized by the client, hex-encoded.
    pub client_message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i64,
    pub expires_at: NaiveDateTime,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/opaque/register-start", post(register_start))
        .route("/api/auth/opaque/register-finish", post(register_finish))
        .route("/api/auth/opaque/login-start", post(login_start))
        .route("/api/auth/opaque/login-finish", post(login_finish))
        .route("/api/auth/logout", post(logout))
}

/// Round 1 of registration.
/// Client sends a blinded password; server returns its public key and OPRF
/// evaluation. No password ever crosses the wire.
///
/// # Errors
///
/// - `ApiError::UnprocessableEntity`: If `client_message` is not valid hex or
///   cannot be deserialized as an OPAQUE `RegistrationRequest`.
/// - `ApiError::Internal`: If the OPAQUE registration start operation fails.
#[utoipa::path(
    post,
    path = "/api/auth/opaque/register-start",
    request_body = OpaqueRegisterStartRequest,
    responses(
        (status = 200, description = "Registration challenge", body = OpaqueRegisterStartResponse),
        (status = 409, description = "Email or handle taken", body = ErrorBody),
        (status = 422, description = "Validation error", body = ErrorBody),
    ),
    tag = "auth"
)]
pub async fn register_start(
    State(state): State<AppState>,
    Json(body): Json<OpaqueRegisterStartRequest>,
) -> Result<Json<OpaqueRegisterStartResponse>, ApiError> {
    let raw = hex::decode(&body.client_message)
        .map_err(|_| ApiError::UnprocessableEntity("client_message is not valid hex.".into()))?;

    let client_msg =
        RegistrationRequest::<DefaultCipherSuite>::deserialize(&raw).map_err(|_| {
            ApiError::UnprocessableEntity("Invalid OPAQUE registration request.".into())
        })?;

    // Server is stateless here - start() only needs the setup + credential id.
    let result = ServerRegistration::<DefaultCipherSuite>::start(
        &state.server_setup,
        client_msg,
        body.email.as_bytes(),
    )
    .map_err(|e| ApiError::Internal(format!("OPAQUE registration start failed: {e}")))?;

    let registration_id = generate_token();
    state
        .store_pending_registration(&registration_id, body.email, body.handle)
        .await;

    Ok(Json(OpaqueRegisterStartResponse {
        registration_id,
        server_message: hex::encode(result.message.serialize()),
    }))
}

/// Round 2 of registration. Client sends the encrypted envelope; server stores
/// it. No password was ever visible to the server.
///
/// # Errors
///
/// - `ApiError::UnprocessableEntity`: If the registration session has expired
///   or is not found, `client_message` is not valid hex, or the OPAQUE upload
///   cannot be deserialized.
/// - `ApiError::Conflict`: If the email or handle is already taken.
/// - `ApiError::Internal`: If the database insert fails unexpectedly.
#[utoipa::path(
    post,
    path = "/api/auth/opaque/register-finish",
    request_body = OpaqueRegisterFinishRequest,
    responses(
        (status = 201, description = "User registered", body = User),
        (status = 422, description = "Validation error", body = ErrorBody),
    ),
    tag = "auth"
)]
pub async fn register_finish(
    State(state): State<AppState>,
    Json(body): Json<OpaqueRegisterFinishRequest>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    let pending = state
        .pop_pending_registration(&body.registration_id)
        .await
        .ok_or_else(|| {
            ApiError::UnprocessableEntity("Registration session expired or not found.".into())
        })?;

    let raw = hex::decode(&body.client_message)
        .map_err(|_| ApiError::UnprocessableEntity("client_message is not valid hex.".into()))?;

    let upload = RegistrationUpload::<DefaultCipherSuite>::deserialize(&raw)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid OPAQUE upload.".into()))?;

    // finish() is infallible - it just packages the envelope.
    let server_record = ServerRegistration::<DefaultCipherSuite>::finish(upload);
    let opaque_record = server_record.serialize().to_vec();

    let conn = state.pool.get().await?;
    let new_user = NewUser {
        email: pending.email,
        handle: pending.handle,
        opaque_record,
    };

    let user: User = conn
        .interact(|conn| {
            conn.transaction(|conn| -> Result<User, diesel::result::Error> {
                let user: User = diesel::insert_into(users_schema::dsl::users::table())
                    .values(new_user)
                    .returning(User::as_returning())
                    .get_result(conn)?;

                // Create the corresponding displayed_users row so the FK
                // constraints on guild_messages.author_id and
                // direct_messages.author_id are satisfied from the moment the
                // user is created.
                diesel::insert_into(displayed_users_schema::table)
                    .values((
                        displayed_users_schema::user_id.eq(Some(user.id)),
                        displayed_users_schema::display_name.eq(&user.handle),
                    ))
                    .execute(conn)?;

                Ok(user)
            })
        })
        .await??;

    Ok((StatusCode::CREATED, Json(user)))
}

/// Round 1 of login. Returns an OPRF evaluation + server ephemeral key.
/// Never reveals whether the email exists (constant-time fake response on miss).
///
/// # Errors
///
/// - `ApiError::UnprocessableEntity`: If `client_message` is not valid hex or
///   cannot be deserialized as an OPAQUE `CredentialRequest`.
/// - `ApiError::Internal`: If the database lookup or OPAQUE login start fails.
#[utoipa::path(
    post,
    path = "/api/auth/opaque/login-start",
    request_body = OpaqueLoginStartRequest,
    responses(
        (status = 200, description = "Login challenge", body = OpaqueLoginStartResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
    ),
    tag = "auth"
)]
pub async fn login_start(
    State(state): State<AppState>,
    Json(body): Json<OpaqueLoginStartRequest>,
) -> Result<Json<OpaqueLoginStartResponse>, ApiError> {
    let raw = hex::decode(&body.client_message)
        .map_err(|_| ApiError::UnprocessableEntity("client_message is not valid hex.".into()))?;

    let client_msg = CredentialRequest::<DefaultCipherSuite>::deserialize(&raw)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid OPAQUE credential request.".into()))?;

    // Look up the user's stored record.
    let conn = state.pool.get().await?;
    let email = body.email.clone();
    let user: Option<User> = conn
        .interact(move |conn| {
            users_schema::dsl::users
                .filter(users_schema::dsl::email.eq(&email))
                .select(User::as_select())
                .first(conn)
                .optional()
        })
        .await?
        .map_err(ApiError::internal)?;

    // Deserialize the record if found; pass None otherwise.
    // opaque-ke will generate a fake response for the None case so timing
    // is identical whether or not the email exists.
    let password_file = match &user {
        Some(u) => {
            let record = ServerRegistration::<DefaultCipherSuite>::deserialize(&u.opaque_record)
                .map_err(|_| ApiError::Internal("Corrupt OPAQUE record in DB.".into()))?;
            Some(record)
        }
        None => None,
    };

    let result = ServerLogin::start(
        &mut OsRng,
        &state.server_setup,
        password_file,
        client_msg,
        body.email.as_bytes(),
        ServerLoginParameters::default(),
    )
    .map_err(|e| ApiError::Internal(format!("OPAQUE login start failed: {e}")))?;

    // Only store state if the user actually exists; reject on finish otherwise.
    let login_id = generate_token();
    if let Some(u) = user {
        state
            .store_pending_login(&login_id, u.id, result.state)
            .await;
    }

    // If user doesn't exist we still return a well-formed (fake) response -
    // the handshake will simply fail at finish with a generic error.
    Ok(Json(OpaqueLoginStartResponse {
        login_id,
        server_message: hex::encode(result.message.serialize()),
    }))
}

/// Round 2 of login. Verifies the client's proof; issues a session token.
///
/// # Errors
///
/// - `ApiError::UnprocessableEntity`: If `client_message` is not valid hex.
/// - `ApiError::Unauthorized`: If the `CredentialFinalization` cannot be
///   deserialized, the login session has expired or is not found, or the
///   OPAQUE proof verification fails (wrong password).
/// - `ApiError::Internal`: If session insertion into the database fails.
#[utoipa::path(
    post,
    path = "/api/auth/opaque/login-finish",
    request_body = OpaqueLoginFinishRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
    ),
    tag = "auth"
)]
pub async fn login_finish(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<OpaqueLoginFinishRequest>,
) -> Result<Response, ApiError> {
    let raw = hex::decode(&body.client_message)
        .map_err(|_| ApiError::UnprocessableEntity("client_message is not valid hex.".into()))?;

    let finalization = CredentialFinalization::<DefaultCipherSuite>::deserialize(&raw)
        .map_err(|_| ApiError::Unauthorized("Invalid credentials.".into()))?;

    let pending = state
        .pop_pending_login(&body.login_id)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials.".into()))?;

    // `finish()` fails if and only if the password was wrong.
    ServerLogin::finish(
        pending.server_login,
        finalization,
        ServerLoginParameters::default(),
    )
    .map_err(|_| ApiError::Unauthorized("Invalid credentials.".into()))?;

    issue_session(pending.user_id, jar, &state.pool).await
}

/// Logs out the authenticated user by invalidating their session.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the session token is missing, invalid, or
///   expired (enforced by the `AuthUser` extractor).
/// - `ApiError::Internal`: If the database delete operation fails.
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 204, description = "Session invalidated"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "auth"
)]
pub async fn logout(auth: AuthUser, State(state): State<AppState>) -> Result<Response, ApiError> {
    let conn = state.pool.get().await?;
    let session_id = auth.session.id;

    conn.interact(move |conn| {
        diesel::delete(
            sessions_schema::dsl::sessions.filter(sessions_schema::dsl::id.eq(session_id)),
        )
        .execute(conn)
    })
    .await??;

    let cookie = Cookie::build(("session_token", ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::ZERO)
        .build();

    Ok((StatusCode::NO_CONTENT, CookieJar::new().add(cookie)).into_response())
}

async fn issue_session(
    user_id: i64,
    jar: CookieJar,
    pool: &deadpool_diesel::postgres::Pool,
) -> Result<Response, ApiError> {
    let token = generate_token();
    let expires_at = (Utc::now() + Duration::days(SESSION_DURATION_DAYS)).naive_utc();
    let new_session = NewSession {
        token: token.clone(),
        user_id,
        expires_at,
    };

    let conn = pool.get().await?;
    conn.interact(move |conn| {
        diesel::insert_into(sessions_schema::dsl::sessions::table())
            .values(new_session)
            .execute(conn)
    })
    .await??;

    let cookie = Cookie::build(("session_token", token.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::days(SESSION_DURATION_DAYS))
        .build();

    Ok((
        StatusCode::OK,
        jar.add(cookie),
        Json(AuthResponse {
            token,
            user_id,
            expires_at,
        }),
    )
        .into_response())
}

fn generate_token() -> String {
    use rand::RngExt;
    use std::fmt::Write;

    let bytes: [u8; 32] = rand::rng().random();

    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}
