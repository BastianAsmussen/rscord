use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper, associations::HasTable};

use super::errors::ApiError;
use crate::db::{
    models::users::{NewUser, UpdateUser, User},
    schema::users as users_schema,
};

type Pool = deadpool_diesel::postgres::Pool;

/// Returns the `/api/users` router with all user CRUD routes.
pub fn routes() -> Router<Pool> {
    Router::new()
        .route("/api/users", post(create_user).get(list_users))
        .route(
            "/api/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
}

/// POST /api/users
///
/// Create a new user.
async fn create_user(
    State(pool): State<Pool>,
    Json(payload): Json<NewUser>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    let conn = pool.get().await?;

    let user: User = conn
        .interact(|conn| {
            diesel::insert_into(users_schema::dsl::users::table())
                .values(payload)
                .returning(User::as_returning())
                .get_result(conn)
        })
        .await??;

    Ok((StatusCode::CREATED, Json(user)))
}

/// GET /api/users
///
/// List all users.
async fn list_users(State(pool): State<Pool>) -> Result<Json<Vec<User>>, ApiError> {
    let conn = pool.get().await?;

    let all_users: Vec<User> = conn
        .interact(|conn| {
            users_schema::dsl::users
                .select(User::as_select())
                .load(conn)
        })
        .await??;

    Ok(Json(all_users))
}

/// GET /api/users/:id
///
/// Get a single user by ID.
async fn get_user(
    State(pool): State<Pool>,
    Path(user_id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    let conn = pool.get().await?;

    let user: User = conn
        .interact(move |conn| {
            users_schema::dsl::users
                .filter(users_schema::dsl::id.eq(user_id))
                .select(User::as_select())
                .first(conn)
        })
        .await?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ApiError::NotFound(format!("User {user_id} not found"))
            }
            other => other.into(),
        })?;

    Ok(Json(user))
}

/// PUT /api/users/:id
///
/// Update an existing user.
async fn update_user(
    State(pool): State<Pool>,
    Path(user_id): Path<i64>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<User>, ApiError> {
    let conn = pool.get().await?;

    let user: User = conn
        .interact(move |conn| {
            diesel::update(users_schema::dsl::users.filter(users_schema::dsl::id.eq(user_id)))
                .set(&payload)
                .returning(User::as_returning())
                .get_result(conn)
        })
        .await?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ApiError::NotFound(format!("User {user_id} not found"))
            }
            other => other.into(),
        })?;

    Ok(Json(user))
}

/// DELETE /api/users/:id
///
/// Delete a user by ID.
async fn delete_user(
    State(pool): State<Pool>,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;

    let rows_deleted: usize = conn
        .interact(move |conn| {
            diesel::delete(users_schema::dsl::users.filter(users_schema::dsl::id.eq(user_id)))
                .execute(conn)
        })
        .await??;

    if rows_deleted == 0 {
        return Err(ApiError::NotFound(format!("User {user_id} not found")));
    }

    Ok(StatusCode::NO_CONTENT)
}
