use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper, associations::HasTable};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{
    auth_extractor::AuthUser,
    errors::ApiError,
    password::{hash_password, verify_password},
};
use crate::{
    api::errors::ErrorBody,
    db::{
        models::{
            sessions::NewSession,
            users::{NewUser, User},
        },
        schema::{sessions as sessions_schema, users as users_schema},
    },
};

type Pool = deadpool_diesel::postgres::Pool;

/// Session lifetime: 30 days.
const SESSION_DURATION_DAYS: i64 = 30;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub handle: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i64,
    pub expires_at: NaiveDateTime,
}

pub fn routes() -> Router<Pool> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
}

/// Registers a new user.
///
/// POST /api/auth/register
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::UnprocessableEntity`: If the password is less than the required length.
/// - `ApiError::Internal`: If an error occurs while hashing the password.
/// - `ApiError::Conflict`: If the user creation fails due to a constraint violation in the database.
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = User),
        (status = 409, description = "Email or handle taken", body = ErrorBody),
        (status = 422, description = "Validation error", body = ErrorBody),
    ),
    tag = "auth"
)]
pub async fn register(
    State(pool): State<Pool>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    if body.password.len() < 8 {
        return Err(ApiError::UnprocessableEntity(
            "Password must be at least 8 characters.".into(),
        ));
    }

    let digest = hash_password(&body.password)?;
    let new_user = NewUser {
        email: body.email,
        password_digest: digest,
        handle: body.handle,
    };

    let conn = pool.get().await?;

    let user: User = conn
        .interact(|conn| {
            diesel::insert_into(users_schema::dsl::users::table())
                .values(new_user)
                .returning(User::as_returning())
                .get_result(conn)
        })
        .await??;

    Ok((StatusCode::CREATED, Json(user)))
}

/// Authenticates a user with their email and password.
///
/// POST /api/auth/login
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::Unauthorized`: If the email or password provided is invalid.
/// - `ApiError::Internal`: If an error occurs while querying the database.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
    ),
    tag = "auth"
)]
pub async fn login(
    State(pool): State<Pool>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let conn = pool.get().await?;

    let email = body.email.clone();
    let user: User = conn
        .interact(move |conn| {
            users_schema::dsl::users
                .filter(users_schema::dsl::email.eq(&email))
                .select(User::as_select())
                .first(conn)
        })
        .await?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ApiError::Unauthorized("Invalid email or password.".into())
            }
            other => ApiError::internal(other),
        })?;

    if !verify_password(&body.password, &user.password_digest)? {
        return Err(ApiError::Unauthorized("Invalid email or password.".into()));
    }

    let token = generate_token();
    let expires_at = (Utc::now() + Duration::days(SESSION_DURATION_DAYS)).naive_utc();

    let new_session = NewSession {
        token: token.clone(),
        user_id: user.id,
        expires_at,
    };

    conn.interact(move |conn| {
        diesel::insert_into(sessions_schema::dsl::sessions::table())
            .values(new_session)
            .execute(conn)
    })
    .await??;

    let cookie = Cookie::build(("session_token", token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(SESSION_DURATION_DAYS))
        .build();

    let body = AuthResponse {
        token,
        user_id: user.id,
        expires_at,
    };

    Ok((StatusCode::OK, jar.add(cookie), Json(body)).into_response())
}

/// Logs out the authenticated user by invalidating their session.
///
/// POST /api/auth/logout
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::Internal`: If an error occurs during the session deletion process.
/// - `ApiError::NotFound`: If the session token could not be matched to a valid session.
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 204, description = "Session invalidated"),
        (status = 401, description = "Unauthorized", body = ErrorBody)
    ),
    security(("session_token" = [])),
    tag = "auth"
)]
pub async fn logout(auth: AuthUser, State(pool): State<Pool>) -> Result<Response, ApiError> {
    let conn = pool.get().await?;

    let session_id = auth.session.id;

    conn.interact(move |conn| {
        diesel::delete(
            sessions_schema::dsl::sessions.filter(sessions_schema::dsl::id.eq(session_id)),
        )
        .execute(conn)
    })
    .await??;

    let jar = CookieJar::new();
    let cookie = Cookie::build(("session_token", ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::ZERO)
        .build();

    Ok((StatusCode::NO_CONTENT, jar.add(cookie)).into_response())
}

/// Generate a 64-character hex token (256 bits of entropy).
fn generate_token() -> String {
    use std::fmt::Write;

    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();

    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");

        s
    })
}
