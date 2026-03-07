use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper, associations::HasTable};

use super::{auth_extractor::AuthUser, errors::ApiError};
use crate::{
    api::{errors::ErrorBody, opaque::AppState},
    db::{
        models::users::{NewUser, UpdateUser, User},
        schema::users as users_schema,
    },
};

type Pool = deadpool_diesel::postgres::Pool;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users", post(create_user).get(list_users))
        .route(
            "/api/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
}

/// Create a new user (admin functionality).
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::Conflict`: If the provided user data violates database constraints.
/// - `ApiError::Internal`: If an error occurs during the database operation.
#[utoipa::path(
    post,
    path = "/api/users",
    request_body = NewUser,
    responses(
        (status = 201, description = "User created", body = User),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 409, description = "Conflict", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "users"
)]
pub async fn create_user(
    _auth: AuthUser,
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

/// Retrieves a list of all users.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::Internal`: If an error occurs during the database query execution.
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of users", body = Vec<User>),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "users"
)]
pub async fn list_users(
    _auth: AuthUser,
    State(pool): State<Pool>,
) -> Result<Json<Vec<User>>, ApiError> {
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

/// Retrieves a user by their unique ID.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::NotFound`: If the user with the given ID is not found in the database.
/// - `ApiError::Internal`: If an error occurs during the database query execution.
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "User not found", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "users"
)]
pub async fn get_user(
    _auth: AuthUser,
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

/// Updates an existing user's data by their unique ID.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::NotFound`: If no user exists with the given ID.
/// - `ApiError::Internal`: If an error occurs during the update operation in the database.
/// - `ApiError::Conflict`: If the updated data violates database constraints.
#[utoipa::path(
    put,
    path = "/api/users/{id}",
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "User updated", body = User),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "User not found", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "users"
)]
pub async fn update_user(
    _auth: AuthUser,
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

/// Deletes a user by their unique ID.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::NotFound`: If the user with the given ID is not found.
/// - `ApiError::Internal`: If an error occurs during the database operation.
#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "User not found", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "users"
)]
pub async fn delete_user(
    _auth: AuthUser,
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
