use super::errors::ApiError;
use crate::api::auth_extractor::AuthUser;
use crate::api::opaque::AppState;
use crate::db::{models::push_tokens::NewPushToken, schema::push_tokens};
use axum::routing::delete;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, associations::HasTable};

type Pool = deadpool_diesel::postgres::Pool;

/// Returns the `/api/push-token` create and delete routes for push token.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/push-token", post(add_push_token))
        .route("/api/push-token/{token}", delete(remove_push_token))
}

/// POST /api/push-token
///
/// Add a push token.
#[utoipa::path(
    post,
    path = "/api/push-token",
    request_body = NewPushToken,
    responses(
        (status = 204, description = "push token uploaded successfully",),
        (status = 409, description = "duplicate Token",),
    ),
    security(("session_token" = [])),
    tag = "push_tokens"
)]
// TODO: Add proper authentication once login has been merged into master.
async fn add_push_token(
    auth: AuthUser,
    State(pool): State<Pool>,
    Json(payload): Json<NewPushToken>,
) -> Result<StatusCode, ApiError> {
    if auth.session.user_id != payload.user_id {
        return Err(ApiError::Forbidden(
            "Unable create push token for other users".into(),
        ));
    }

    let conn = pool.get().await?;

    conn.interact(|conn| {
        diesel::insert_into(push_tokens::dsl::push_tokens::table())
            .values(payload)
            .execute(conn)
    })
    .await??;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/push-token:token
///
/// Removes a push token.
#[utoipa::path(
    delete,
    path = "/api/push-token/{token}",
    params(("token" = String, Path, description = "The push token to be deleted")),
    responses((status = 204, description = "Token deleted")),
    responses((status = 404, description = "Token could not be found")),
    security(("session_token" = [])),
    tag = "push_tokens"
)]
// TODO: Add proper authentication once login has been merged into master.
async fn remove_push_token(
    _auth: AuthUser,
    State(pool): State<Pool>,
    Path(token): Path<String>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;
    let token_for_error = token.clone();

    let rows_deleted: usize = conn
        .interact(move |conn| {
            diesel::delete(push_tokens::dsl::push_tokens.filter(push_tokens::dsl::token.eq(token)))
                .execute(conn)
        })
        .await??;

    if rows_deleted == 0 {
        return Err(ApiError::NotFound(format!(
            "Token {token_for_error} not found!"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}
