use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, delete, put},
    Json, Router,
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper, Connection};
use diesel::prelude::*;

use super::{auth_extractor::AuthUser, errors::ApiError};
use crate::api::errors::ErrorBody;
use crate::api::opaque::AppState;
use crate::db::{
    models::roles::{Role, NewRole, UpdateRole},
    schema::{roles, guilds, guild_members},
};
use crate::db::models::guilds::Guild;
use crate::db::models::roles::RoleSummary;

type Pool = deadpool_diesel::postgres::Pool;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/guilds/{id}/roles", post(create_role).get(list_roles))
        .route("/api/guilds/{id}/roles/{role_id}", delete(delete_role).put(update_role))
}

/// Create a Role
///
/// Only the Guild Owner can create roles for a guild.
///
/// # Errors
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not the owner.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/guilds/{id}/roles",
    params(("id" = i64, Path, description = "Guild ID")),
    request_body = NewRole,
    responses(
        (status = 201, description = "Role created", body = Role),
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Guild not found"),
    ),
    security(("session_token" = [])),
    tag = "roles"
)]
pub async fn create_role(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
    Json(mut payload): Json<NewRole>,
) -> Result<(StatusCode, Json<Role>), ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let role = conn.interact(move |conn| {
        conn.transaction::<Role, diesel::result::Error, _>(|inner_conn| {
            let is_owner = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(user_id))
                .first::<Guild>(inner_conn)
                .optional()?
                .is_some();

            if !is_owner {
                return Err(diesel::result::Error::NotFound);
            }

            payload.guild_id = guild_id;
            diesel::insert_into(roles::table)
                .values(&payload)
                .returning(Role::as_returning())
                .get_result(inner_conn)
        })
    }).await?.map_err(|_| ApiError::NotFound("Only the owner can create roles or guild not found".into()))?;

    Ok((StatusCode::CREATED, Json(role)))
}

/// List Guild Roles
///
/// Users must be a member of the guild to view its roles.
///
/// # Errors
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not a member.
/// - `ApiError::Internal`: If the database query fails.
#[utoipa::path(
    get,
    path = "/api/guilds/{id}/roles",
    params(("id" = i64, Path, description = "Guild ID")),
    responses((status = 200, body = Vec<RoleSummary>)),
    security(("session_token" = [])),
    tag = "roles"
)]
pub async fn list_roles(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
) -> Result<Json<Vec<RoleSummary>>, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let roles_list = conn.interact(move |conn| {
        let is_member = guild_members::table
            .filter(guild_members::guild_id.eq(guild_id))
            .filter(guild_members::user_id.eq(user_id))
            .count()
            .get_result::<i64>(conn)? > 0;

        if !is_member {
            return Err(diesel::result::Error::NotFound);
        }

        roles::table
            .filter(roles::guild_id.eq(guild_id))
            .order(roles::priority.desc())
            .select(RoleSummary::as_select())
            .load::<RoleSummary>(conn)
    })
        .await?
        .map_err(|_| ApiError::NotFound("Not a member of this guild".into()))?;

    Ok(Json(roles_list))
}

/// Update a Role
///
/// Only the Guild Owner can update roles.
///
/// # Errors
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild/role does not exist or the user is not the owner.
/// - `ApiError::Internal`: If the database update fails.
#[utoipa::path(
    put,
    path = "/api/guilds/{id}/roles/{role_id}",
    params(
        ("id" = i64, Path, description = "Guild ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    request_body = UpdateRole,
    responses((status = 200, body = Role)),
    security(("session_token" = [])),
    tag = "roles"
)]
pub async fn update_role(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path((guild_id, role_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateRole>,
) -> Result<Json<Role>, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let updated_role = conn.interact(move |conn| {
        conn.transaction::<Role, diesel::result::Error, _>(|inner_conn| {
            let is_owner = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(user_id))
                .execute(inner_conn)? > 0;

            if !is_owner {
                return Err(diesel::result::Error::NotFound);
            }

            diesel::update(roles::table.filter(roles::id.eq(role_id)).filter(roles::guild_id.eq(guild_id)))
                .set(&payload)
                .returning(Role::as_returning())
                .get_result(inner_conn)
        })
    }).await?.map_err(|_| ApiError::NotFound("Unauthorized or role not found".into()))?;

    Ok(Json(updated_role))
}

/// Delete a Role
///
/// Only the Guild Owner can delete roles.
///
/// # Errors
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild/role does not exist or the user is not the owner.
/// - `ApiError::Internal`: If the database deletion fails.
#[utoipa::path(
    delete,
    path = "/api/guilds/{id}/roles/{role_id}",
    params(
        ("id" = i64, Path, description = "Guild ID"),
        ("role_id" = i64, Path, description = "Role ID")
    ),
    responses((status = 204, description = "Role deleted")),
    security(("session_token" = [])),
    tag = "roles"
)]
pub async fn delete_role(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path((guild_id, role_id)): Path<(i64, i64)>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    conn.interact(move |conn| {
        conn.transaction::<_, diesel::result::Error, _>(|inner_conn| {
            let is_owner = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(user_id))
                .execute(inner_conn)? > 0;

            if !is_owner {
                return Err(diesel::result::Error::NotFound);
            }

            diesel::delete(roles::table.filter(roles::id.eq(role_id)).filter(roles::guild_id.eq(guild_id)))
                .execute(inner_conn)
        })
    }).await?.map_err(|_| ApiError::NotFound("Unauthorized or role not found".into()))?;

    Ok(StatusCode::NO_CONTENT)
}