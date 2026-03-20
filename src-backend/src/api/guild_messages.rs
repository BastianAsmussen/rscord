use crate::api::auth_extractor::AuthUser;
use crate::api::errors::ApiError;
use crate::api::opaque::AppState;
use crate::db::models::guild_messages::{GuildMessage, NewGuildMessage};
use crate::db::schema::{
    channels, displayed_users, guild_members, guild_messages, sessions, users,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use diesel::prelude::*;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/channels/{channel_id}/messages",
            post(send_guild_message),
        )
        .route(
            "/api/channels/{channel_id}/messages",
            get(get_guild_messages),
        )
}

/// Distinguishes the reasons a guild-message operation can fail inside the
/// database closure so the outer code can map each to the correct HTTP status.
#[derive(Debug)]
enum MessageError {
    InvalidSession,
    ChannelNotFound,
    NotAMember,
    Database(diesel::result::Error),
}

/// Send a Guild Message
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::NotFound`: If the channel does not exist.
/// - `ApiError::Forbidden`: If the user is not a member of the guild.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/channels/{channel_id}/messages",
    responses(
        (status = 201, description = "Message sent", body = GuildMessage),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Channel not found")
    ),
    params(("channel_id" = i64, Path, description = "Channel ID")),
    security(("session_token" = [])),
)]
pub async fn send_guild_message(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<i64>,
    Json(payload): Json<NewGuildMessage>,
) -> Result<(StatusCode, Json<GuildMessage>), ApiError> {
    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;
    let session_id = auth.session.id;

    let message = conn
        .interact(move |conn| {
            let session_exists = sessions::table
                .find(session_id)
                .first::<crate::db::models::sessions::Session>(conn)
                .optional()
                .map_err(MessageError::Database)?
                .is_some();

            if !session_exists {
                return Err(MessageError::InvalidSession);
            }

            let guild_id = channels::table
                .find(channel_id)
                .select(channels::guild_id.assume_not_null())
                .first::<i64>(conn)
                .map_err(|e| match e {
                    diesel::result::Error::NotFound => MessageError::ChannelNotFound,
                    other => MessageError::Database(other),
                })?;

            let is_member = guild_members::table
                .filter(guild_members::guild_id.eq(guild_id))
                .filter(guild_members::user_id.eq(user_id))
                .count()
                .get_result::<i64>(conn)
                .map_err(MessageError::Database)?
                > 0;

            if !is_member {
                return Err(MessageError::NotAMember);
            }

            // Resolve the displayed_users.id for this user (the FK target for
            // guild_messages.author_id). Created atomically at registration,
            // but may be missing for users who registered before this was added.
            let displayed_user_id: i64 = match displayed_users::table
                .filter(displayed_users::user_id.eq(user_id))
                .select(displayed_users::id)
                .first::<i64>(conn)
                .optional()
                .map_err(MessageError::Database)?
            {
                Some(id) => id,
                None => {
                    // Backfill: create the missing displayed_users row using
                    // the user's handle from the users table.
                    let handle: String = users::table
                        .find(user_id)
                        .select(users::user_handle)
                        .first::<String>(conn)
                        .map_err(MessageError::Database)?;

                    diesel::insert_into(displayed_users::table)
                        .values((
                            displayed_users::user_id.eq(Some(user_id)),
                            displayed_users::display_name.eq(&handle),
                        ))
                        .on_conflict(displayed_users::user_id)
                        .do_nothing()
                        .execute(conn)
                        .map_err(MessageError::Database)?;

                    // Re-fetch the id (handles the race where another request
                    // inserted concurrently via ON CONFLICT DO NOTHING).
                    displayed_users::table
                        .filter(displayed_users::user_id.eq(user_id))
                        .select(displayed_users::id)
                        .first::<i64>(conn)
                        .map_err(MessageError::Database)?
                }
            };

            diesel::insert_into(guild_messages::table)
                .values((
                    guild_messages::author_id.eq(displayed_user_id),
                    guild_messages::channel_id.eq(channel_id),
                    guild_messages::reply_to_id.eq(payload.reply_to_id),
                    guild_messages::contents.eq(payload.contents),
                ))
                .returning(GuildMessage::as_returning())
                .get_result::<GuildMessage>(conn)
                .map_err(MessageError::Database)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| match e {
            MessageError::InvalidSession => ApiError::Unauthorized("Invalid session".into()),
            MessageError::ChannelNotFound => ApiError::NotFound("Channel not found".into()),
            MessageError::NotAMember => ApiError::Forbidden("Not a member".into()),
            MessageError::Database(e) => ApiError::Internal(e.to_string()),
        })?;

    drop(state.tx.send(message.clone()));
    Ok((StatusCode::CREATED, Json(message)))
}

/// Get Message History
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::NotFound`: If the channel does not exist.
/// - `ApiError::Forbidden`: If the user is not a member of the guild.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    get,
    path = "/api/channels/{channel_id}/messages",
    responses(
        (status = 200, description = "Message history", body = Vec<GuildMessage>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Channel not found")
    ),
    params(("channel_id" = i64, Path, description = "Channel ID"))
)]
pub async fn get_guild_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<i64>,
) -> Result<Json<Vec<GuildMessage>>, ApiError> {
    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;
    let session_id = auth.session.id;

    let messages = conn
        .interact(move |conn| {
            let session_exists = sessions::table
                .find(session_id)
                .first::<crate::db::models::sessions::Session>(conn)
                .optional()
                .map_err(MessageError::Database)?
                .is_some();

            if !session_exists {
                return Err(MessageError::InvalidSession);
            }

            let guild_id = channels::table
                .find(channel_id)
                .select(channels::guild_id.assume_not_null())
                .first::<i64>(conn)
                .map_err(|e| match e {
                    diesel::result::Error::NotFound => MessageError::ChannelNotFound,
                    other => MessageError::Database(other),
                })?;

            let is_member = guild_members::table
                .filter(guild_members::guild_id.eq(guild_id))
                .filter(guild_members::user_id.eq(user_id))
                .count()
                .get_result::<i64>(conn)
                .map_err(MessageError::Database)?
                > 0;

            if !is_member {
                return Err(MessageError::NotAMember);
            }

            guild_messages::table
                .filter(guild_messages::channel_id.eq(channel_id))
                .order(guild_messages::created_at.desc())
                .limit(50)
                .select(GuildMessage::as_select())
                .load::<GuildMessage>(conn)
                .map_err(MessageError::Database)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| match e {
            MessageError::InvalidSession => ApiError::Unauthorized("Invalid session".into()),
            MessageError::ChannelNotFound => ApiError::NotFound("Channel not found".into()),
            MessageError::NotAMember => {
                ApiError::Forbidden("Not a member of this guild".into())
            }
            MessageError::Database(e) => ApiError::Internal(e.to_string()),
        })?;

    Ok(Json(messages))
}
