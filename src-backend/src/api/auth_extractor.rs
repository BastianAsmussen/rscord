use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};

use super::errors::ApiError;
use crate::db::{models::sessions::Session, schema::sessions as sessions_schema};

type Pool = deadpool_diesel::postgres::Pool;

/// Extracts the authenticated user's session from either:
/// - a `session_token` cookie, or
/// - an `Authorization: Bearer <token>` header.
pub struct AuthUser {
    /// The validated session (contains `user_id`, etc.).
    pub session: Session,
}

impl FromRequestParts<Pool> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, pool: &Pool) -> Result<Self, Self::Rejection> {
        // 1. Extract the token from cookie or header.
        let token = extract_token(parts)?;

        // 2. Get a DB connection directly from the pool state.
        let conn = pool.get().await?;

        // 3. Look up the session.
        let token_clone = token.clone();
        let session: Session = conn
            .interact(move |conn| {
                sessions_schema::dsl::sessions
                    .filter(sessions_schema::dsl::token.eq(&token_clone))
                    .select(Session::as_select())
                    .first(conn)
            })
            .await?
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    ApiError::Unauthorized("Invalid or expired session token.".into())
                }
                other => ApiError::internal(other),
            })?;

        // 4. Check expiry.
        if session.expires_at < Utc::now().naive_utc() {
            return Err(ApiError::Unauthorized(
                "Session has expired. Please log in again.".into(),
            ));
        }

        Ok(Self { session })
    }
}

/// Pull the bearer token from the cookie jar or the `Authorization` header.
fn extract_token(parts: &Parts) -> Result<String, ApiError> {
    // Try the cookie first.
    let jar = CookieJar::from_headers(&parts.headers);
    if let Some(cookie) = jar.get("session_token") {
        let value = cookie.value().trim();
        if !value.is_empty() {
            return Ok(value.to_owned());
        }
    }

    // Fall back to `Authorization: Bearer <token>`.
    if let Some(auth_header) = parts.headers.get("authorization") {
        let header_str = auth_header
            .to_str()
            .map_err(|_| ApiError::Unauthorized("Invalid Authorization header.".into()))?;

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            let token = token.trim();
            if !token.is_empty() {
                return Ok(token.to_owned());
            }
        }
    }

    Err(ApiError::Unauthorized(
        "Missing session token. Provide a `session_token` cookie or `Authorization: Bearer` header.".into(),
    ))
}
