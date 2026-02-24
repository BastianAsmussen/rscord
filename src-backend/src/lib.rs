use axum::{Json, extract::State, http::StatusCode};
use diesel::{RunQueryDsl, SelectableHelper, associations::HasTable};

use crate::db::{
    models::{NewUser, User},
    schema::users::dsl::users,
};

pub mod db;

pub async fn create_user(
    State(pool): State<deadpool_diesel::postgres::Pool>,
    Json(payload): Json<NewUser>,
) -> Result<Json<User>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res: User = conn
        .interact(|conn| {
            diesel::insert_into(users::table())
                .values(payload)
                .returning(User::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)? // Handles deadpool interact error
        .map_err(internal_error)?; // Handles diesel query error

    Ok(Json(res))
}

/// Utility function for mapping any error into a `500 Internal Server Error`.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
