use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use axum::routing::delete;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper, associations::HasTable};
use super::errors::ApiError;
use crate::db::{
    models::push_tokens::NewPushToken,
    schema::push_tokens
};

type Pool = deadpool_diesel::postgres::Pool;

/// Returns the `/api/push-token` create and delete routes for push token
pub fn routes() -> Router<Pool> {
    Router::new()
        .route("/api/push-token", post(add_push_token))
        .route("/api/push-token/{token}", delete(remove_push_token))
}

/// POST /api/push-token
///
/// Add a push token
async fn add_push_token(
    State(pool): State<Pool>,
    Json(payload): Json<NewPushToken>,
) -> Result<StatusCode, ApiError> {
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
/// Removes a push token
async fn remove_push_token(
    State(pool): State<Pool>,
    Path(token): Path<String>
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;
    let token_for_error = token.to_string();

    let rows_deleted: usize = conn
        .interact(move |conn| {
            diesel::delete(push_tokens::dsl::push_tokens.filter(push_tokens::dsl::token.eq(token)))
                .execute(conn)
        })
        .await??;

    if rows_deleted == 0 {
        return Err(ApiError::NotFound(format!("User {token_for_error} not found")));
    }

    Ok(StatusCode::NO_CONTENT)
}
