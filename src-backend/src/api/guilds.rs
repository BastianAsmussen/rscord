use axum::routing::get;
use axum::{
    extract::{Path, State}, http::StatusCode,
    routing::{delete, post},
    Json,
    Router,
};
use diesel::prelude::*;
use diesel::{Connection, ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper};

use super::{auth_extractor::AuthUser, errors::ApiError};
use crate::api::errors::ErrorBody;
use crate::api::opaque::AppState;
use crate::db::models::guilds::{GuildMemberWithRoles, UpdateGuild, UpdateGuildChannel};
use crate::db::models::roles::RoleSummary;
use crate::db::schema::{members_roles, roles, users};
use crate::db::{
    models::channels::ChannelType,
    models::guild_channels::{GuildChannel, NewGuildChannel},
    models::guilds::{Guild, GuildSummary, NewGuild, NewGuildMember},
    schema::{guild_channels, guild_members, guilds},
};

type Pool = deadpool_diesel::postgres::Pool;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/guilds", post(create_guild).get(list_my_guilds))
        .route("/api/guilds/{id}", delete(delete_guild).put(update_guild))
        .route("/api/guilds/{id}/join", post(join_guild))
        .route("/api/guilds/{id}/leave", post(leave_guild))
        .route(
            "/api/guilds/{id}/channels",
            post(create_guild_channel).get(get_guild_channels),
        )
        .route("/api/guilds/{id}/channels/{channel_id}", delete(delete_guild_channel).put(update_guild_channel))
        .route("/api/guilds/{id}/members", get(get_guild_members))
}

/// Create Guild
///
/// Creates a new guild:
/// - adds the creator as a member
/// - adds default #general text channel.
///
/// # Errors
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::Internal`: If the transaction fails or the database is unreachable.
#[utoipa::path(
    post,
    path = "/api/guilds",
    request_body = NewGuild,
    responses(
        (status = 201, description = "Guild created", body = Guild),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn create_guild(
    auth: AuthUser,
    State(pool): State<Pool>,
    Json(mut payload): Json<NewGuild>,
) -> Result<(StatusCode, Json<Guild>), ApiError> {
    let conn = pool.get().await?;
    payload.owner_id = auth.session.user_id;
    let user_id = auth.session.user_id;

    let guild: Guild = conn
        .interact(move |conn| {
            conn.transaction::<Guild, diesel::result::Error, _>(|inner_conn| {
                let new_guild: Guild = diesel::insert_into(guilds::table)
                    .values(&payload)
                    .returning(Guild::as_returning())
                    .get_result(inner_conn)?;

                diesel::insert_into(guild_members::table)
                    .values(NewGuildMember {
                        guild_id: new_guild.id,
                        user_id,
                    })
                    .execute(inner_conn)?;

                // Create default #general
                diesel::insert_into(guild_channels::table)
                    .values(NewGuildChannel {
                        guild_id: new_guild.id,
                        name: "general".to_string(),
                        type_: ChannelType::Text,
                        topic: Some("Default text channel".to_string()),
                        position: Some(0),
                    })
                    .execute(inner_conn)?;

                Ok(new_guild)
            })
        })
        .await??;

    Ok((StatusCode::CREATED, Json(guild)))
}

/// Update Guild
///
/// Only the Guild Owner can update the guild settings.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::Forbidden`: If the user tries to delete the last remaining channel.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not the owner.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    put,
    path = "/api/guilds/{id}",
    params(("id" = i64, Path, description = "Guild ID")),
    request_body = UpdateGuild,
    responses(
        (status = 200, description = "Guild updated", body = Guild),
        (status = 404, description = "Guild not found or unauthorized"),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn update_guild(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
    Json(payload): Json<UpdateGuild>,
) -> Result<Json<Guild>, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let updated_guild = conn.interact(move |conn| {
        diesel::update(
            guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(user_id))
        )
            .set(&payload)
            .returning(Guild::as_returning())
            .get_result(conn)
    })
        .await?
        .map_err(|_| ApiError::NotFound("Guild not found or unauthorized".into()))?;

    Ok(Json(updated_guild))
}

/// List User Guilds
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::Internal`: If a database error occurs.
#[utoipa::path(
    get,
    path = "/api/guilds",
    responses(
        (status = 200, description = "List of guilds user is a member of", body = Vec<GuildSummary>),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn list_my_guilds(
    auth: AuthUser,
    State(pool): State<Pool>,
) -> Result<Json<Vec<GuildSummary>>, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let my_guilds = conn
        .interact(move |conn| {
            guilds::table
                .inner_join(guild_members::table.on(guilds::id.eq(guild_members::guild_id)))
                .filter(guild_members::user_id.eq(user_id))
                .select(GuildSummary::as_select())
                .load::<GuildSummary>(conn)
        })
        .await??;

    Ok(Json(my_guilds))
}

/// Join a Guild
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild ID does not exist.
/// - `ApiError::UnprocessableEntity`: If the user is already a member of the guild.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/guilds/{id}/join",
    params(("id" = i64, Path, description = "Guild ID")),
    responses(
        (status = 204, description = "Successfully joined"),
        (status = 404, description = "Guild not found"),
        (status = 422, description = "Already a member"),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn join_guild(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    conn.interact(move |conn| {
        let exists = guilds::table
            .filter(guilds::id.eq(guild_id))
            .select(diesel::dsl::count_star())
            .get_result::<i64>(conn)?
            > 0;

        if !exists {
            return Err(diesel::result::Error::NotFound);
        }

        diesel::insert_into(guild_members::table)
            .values(NewGuildMember { guild_id, user_id })
            .execute(conn)
    })
    .await?
    .map_err(|e| match e {
        diesel::result::Error::NotFound => {
            ApiError::NotFound(format!("Guild {guild_id} not found"))
        }
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => ApiError::UnprocessableEntity("Already a member".into()),
        other => ApiError::internal(other),
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Leave a Guild
///
/// Owners cannot leave: they must delete the guild instead.
///
/// # Errors
///
/// - `ApiError::Forbidden`: If the user is the owner and should use delete instead.
/// - `ApiError::NotFound`: If the guild ID does not exist.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/guilds/{id}/leave",
    params(("id" = i64, Path, description = "Guild ID")),
    responses(
        (status = 204, description = "Successfully left the guild"),
        (status = 403, description = "Owners cannot leave their own guild", body = ErrorBody),
        (status = 404, description = "Guild not found or not a member"),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn leave_guild(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    conn.interact(move |conn| {
        conn.transaction::<_, diesel::result::Error, _>(|inner_conn| {
            let is_owner = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(user_id))
                .count()
                .get_result::<i64>(inner_conn)? > 0;

            if is_owner {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            let rows = diesel::delete(
                guild_members::table
                    .filter(guild_members::guild_id.eq(guild_id))
                    .filter(guild_members::user_id.eq(user_id))
            ).execute(inner_conn)?;

            if rows == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            Ok(())
        })
    })
        .await?
        .map_err(|e| match e {
            diesel::result::Error::RollbackTransaction => {
                ApiError::Forbidden("Owners cannot leave their own guild. Delete the guild instead.".into())
            }
            diesel::result::Error::NotFound => {
                ApiError::NotFound("Guild not found or you are not a member.".into())
            }
            other => ApiError::internal(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete Guild
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not the owner.
/// - `ApiError::Internal`: If the deletion fails or a cascade error occurs.
#[utoipa::path(
    delete,
    path = "/api/guilds/{id}",
    params(("id" = i64, Path, description = "Guild ID")),
    responses((status = 204, description = "Guild deleted")),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn delete_guild(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;

    conn.interact(move |conn| {
        let rows_affected = diesel::delete(
            guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(auth.session.user_id)),
        )
        .execute(conn)?;

        if rows_affected == 0 {
            return Err(diesel::result::Error::NotFound);
        }

        Ok(())
    })
    .await?
    .map_err(|e| match e {
        diesel::result::Error::NotFound => {
            ApiError::NotFound("Guild not found or unauthorized".into())
        }
        other => ApiError::internal(other),
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create Guild Channel
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not the owner.
/// - `ApiError::UnprocessableEntity`: If an invalid channel type (example: DM) is provided.
/// - `ApiError::Internal`: If the database insertion fails.
#[utoipa::path(
    post,
    path = "/api/guilds/{id}/channels",
    params(("id" = i64, Path, description = "Guild ID")),
    request_body = NewGuildChannel,
    responses(
        (status = 201, description = "Channel created", body = GuildChannel),
        (status = 422, description = "Invalid channel type"),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn create_guild_channel(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
    Json(mut payload): Json<NewGuildChannel>,
) -> Result<(StatusCode, Json<GuildChannel>), ApiError> {
    match payload.type_ {
        ChannelType::Text | ChannelType::Voice => (),
        _ => {
            return Err(ApiError::UnprocessableEntity(
                "Invalid channel type for guild".into(),
            ));
        }
    }

    // Inject the guild_id from the Path into the payload
    // since Serde skipped it during deserialization.
    payload.guild_id = guild_id;

    let conn = pool.get().await?;
    let channel = conn.interact(move |conn| {
        conn.transaction::<_, diesel::result::Error, _>(|inner_conn| {
            let owner_count: i64 = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(auth.session.user_id))
                .count()
                .get_result(inner_conn)?;

            if owner_count == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            // If position is null, find the highest existing and add 1
            if payload.position.is_none() {
                let max_pos: Option<i32> = guild_channels::table
                    .filter(guild_channels::guild_id.eq(guild_id))
                    .select(diesel::dsl::max(guild_channels::position))
                    .first(inner_conn)?;

                // If no channels exist, max_pos is None so start at 0
                // - should not be able to happen but to be sure
                payload.position = Some(max_pos.unwrap_or(-1) + 1);
            }

            diesel::insert_into(guild_channels::table)
                .values(&payload)
                .returning(GuildChannel::as_returning())
                .get_result(inner_conn)
        })
    })
        .await?
        .map_err(|_| ApiError::NotFound("Guild not found or unauthorized".into()))?;

    Ok((StatusCode::CREATED, Json(channel)))
}

/// Delete Guild Channel
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::Forbidden`: If the user tries to delete the last remaining channel.
/// - `ApiError::NotFound`: If the guild/channel does not exist or the user is not the owner.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    delete,
    path = "/api/guilds/{id}/channels/{channel_id}",
    params(
        ("id" = i64, Path, description = "Guild ID"),
        ("channel_id" = i64, Path, description = "Channel ID to delete")
    ),
    responses(
        (status = 204, description = "Channel deleted"),
        (status = 403, description = "Forbidden: User is not owner or it's the last channel"),
        (status = 404, description = "Guild or Channel not found")
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn delete_guild_channel(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path((guild_id, channel_id)): Path<(i64, i64)>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;

    conn.interact(move |conn| {
        conn.transaction::<_, diesel::result::Error, _>(|inner_conn| {
            let is_owner = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(auth.session.user_id))
                .count()
                .get_result::<i64>(inner_conn)? > 0;

            if !is_owner {
                return Err(diesel::result::Error::NotFound);
            }

            // Ensure there is more than 1 channel
            let channel_count = guild_channels::table
                .filter(guild_channels::guild_id.eq(guild_id))
                .count()
                .get_result::<i64>(inner_conn)?;

            if channel_count <= 1 {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            let rows = diesel::delete(
                guild_channels::table
                    .filter(guild_channels::id.eq(channel_id))
                    .filter(guild_channels::guild_id.eq(guild_id))
            ).execute(inner_conn)?;

            if rows == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            Ok(())
        })
    })
        .await?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ApiError::NotFound("Guild or Channel not found/unauthorized".into())
            }
            diesel::result::Error::RollbackTransaction => {
                ApiError::Forbidden("Cannot delete the last channel in a guild".into())
            }
            other => ApiError::internal(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update Guild Channel
///
/// Only the Guild Owner can update channel settings.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not a member.
/// - `ApiError::Internal`: If the database query fails.
#[utoipa::path(
    put,
    path = "/api/guilds/{id}/channels/{channel_id}",
    params(
        ("id" = i64, Path, description = "Guild ID"),
        ("channel_id" = i64, Path, description = "Channel ID")
    ),
    request_body = UpdateGuildChannel,
    responses(
        (status = 200, description = "Channel updated", body = GuildChannel),
        (status = 404, description = "Guild/Channel not found or unauthorized"),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn update_guild_channel(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path((guild_id, channel_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateGuildChannel>,
) -> Result<Json<GuildChannel>, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let channel = conn.interact(move |conn| {
        conn.transaction::<GuildChannel, diesel::result::Error, _>(|inner_conn| {
            let is_owner = guilds::table
                .filter(guilds::id.eq(guild_id))
                .filter(guilds::owner_id.eq(user_id))
                .count()
                .get_result::<i64>(inner_conn)? > 0;

            if !is_owner {
                return Err(diesel::result::Error::NotFound);
            }

            diesel::update(
                guild_channels::table
                    .filter(guild_channels::id.eq(channel_id))
                    .filter(guild_channels::guild_id.eq(guild_id))
            )
                .set(&payload)
                .returning(GuildChannel::as_returning())
                .get_result(inner_conn)
        })
    })
        .await?
        .map_err(|_| ApiError::NotFound("Guild or Channel not found/unauthorized".into()))?;

    Ok(Json(channel))
}

/// Get Guild Channels
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not a member.
/// - `ApiError::Internal`: If the database query fails.
#[utoipa::path(
    get,
    path = "/api/guilds/{id}/channels",
    params(("id" = i64, Path, description = "Guild ID")),
    responses((status = 200, body = Vec<GuildChannel>)),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn get_guild_channels(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
) -> Result<Json<Vec<GuildChannel>>, ApiError> {
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let channels_list = conn
        .interact(move |conn| {
            let is_member = guild_members::table
                .filter(guild_members::guild_id.eq(guild_id))
                .filter(guild_members::user_id.eq(user_id))
                .select(diesel::dsl::count_star())
                .get_result::<i64>(conn)?
                > 0;

            if !is_member {
                return Err(diesel::result::Error::NotFound);
            }

            guild_channels::table
                .filter(guild_channels::guild_id.eq(guild_id))
                .order(guild_channels::position.asc())
                .select(GuildChannel::as_select())
                .load::<GuildChannel>(conn)
        })
        .await?
        .map_err(|_| ApiError::NotFound("Guild not found or not a member".into()))?;

    Ok(Json(channels_list))
}

/// Get Guild Channels
///
/// Returns all channels for a specific guild if the user is a member.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user is not logged in.
/// - `ApiError::NotFound`: If the guild does not exist or the user is not a member.
/// - `ApiError::Internal`: If the database query fails.
#[utoipa::path(
    get,
    path = "/api/guilds/{id}/members",
    params(("id" = i64, Path, description = "Guild ID")),
    responses(
        (status = 200, description = "List of members with roles", body = Vec<GuildMemberWithRoles>),
        (status = 404, description = "Guild not found"),
    ),
    security(("session_token" = [])),
    tag = "guilds"
)]
pub async fn get_guild_members(
    auth: AuthUser,
    State(pool): State<Pool>,
    Path(guild_id): Path<i64>,
) -> Result<Json<Vec<GuildMemberWithRoles>>, ApiError> {
    use std::collections::{HashMap};
    let conn = pool.get().await?;
    let user_id = auth.session.user_id;

    let members = conn
        .interact(move |conn| {
            let is_member = guild_members::table
                .filter(guild_members::guild_id.eq(guild_id))
                .filter(guild_members::user_id.eq(user_id))
                .count()
                .get_result::<i64>(conn)?
                > 0;

            if !is_member {
                return Err(diesel::result::Error::NotFound);
            }

            // Fetch all members and their roles
            // Users -> GuildMembers -> MembersRoles (Left Join) -> Roles (Left Join)
            let data = users::table
                .inner_join(guild_members::table.on(guild_members::user_id.eq(users::id)))
                .left_join(
                    members_roles::table.on(members_roles::user_id
                        .eq(users::id)
                        .and(members_roles::guild_id.eq(guild_members::guild_id))),
                )
                .left_join(roles::table.on(roles::id.eq(members_roles::role_id)))
                .filter(guild_members::guild_id.eq(guild_id))
                .select((
                    users::id,
                    users::user_handle,
                    Option::<RoleSummary>::as_select(),
                ))
                .load::<(i64, String, Option<RoleSummary>)>(conn)?;

            let mut member_map: HashMap<i64, GuildMemberWithRoles> = HashMap::new();

            for (u_id, handle, role_opt) in data {
                let entry = member_map.entry(u_id).or_insert(GuildMemberWithRoles {
                    user_id: u_id,
                    user_handle: handle,
                    roles: Vec::new(),
                });

                if let Some(role) = role_opt {
                    entry.roles.push(role);
                }
            }

            Ok(member_map.into_values().collect::<Vec<_>>())
        })
        .await?
        .map_err(|_| ApiError::NotFound("Guild not found or not a member".into()))?;

    Ok(Json(members))
}
