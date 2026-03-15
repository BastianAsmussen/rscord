use crate::api::auth_extractor::AuthUser;
use crate::api::errors::ApiError;
use crate::api::opaque::AppState;
use crate::db::models::messages::{GuildMessage, NewGuildMessage};
use crate::db::schema::{channels, channels_members, guild_members, guild_messages, sessions};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use diesel::prelude::*;
use ApiError::Forbidden;

pub fn dm_routes() -> Router<AppState> {
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

/// Send a Direct Message
///
/// # Errors
///
#[utoipa::path(
    post,
    path = "/api/channels/{channel_id}/messages",
    responses(
        (status = 201, description = "Message sent", body = GuildMessage),
        (status = 403, description = "Forbidden")
    ),
    params(("channel_id" = i64, Path, description = "Channel ID"))
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
                .optional()?
                .is_some();

            if !session_exists {
                return Err(diesel::result::Error::NotFound);
            }

            let guild_id = channels::table
                .find(channel_id)
                .select(channels::guild_id.assume_not_null())
                .first::<i64>(conn)?;

            let is_member = guild_members::table
                .filter(guild_members::guild_id.eq(guild_id))
                .filter(guild_members::user_id.eq(user_id))
                .count()
                .get_result::<i64>(conn)? > 0;

            if !is_member {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            diesel::insert_into(guild_messages::table)
                .values((
                    guild_messages::author_id.eq(user_id),
                    guild_messages::channel_id.eq(channel_id),
                    guild_messages::reply_to_id.eq(payload.reply_to_id),
                    guild_messages::contents.eq(payload.contents),
                ))
                .returning(GuildMessage::as_returning())
                .get_result::<GuildMessage>(conn)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ApiError::Unauthorized("Invalid session".into()),
            diesel::result::Error::RollbackTransaction => ApiError::Forbidden("Not a member".into()),
            _ => ApiError::Internal(e.to_string()),
        })?;

    drop(state.tx.send(message.clone()));
    Ok((StatusCode::CREATED, Json(message)))
}

/// Get Message History
///
/// # Errors
///
#[utoipa::path(
    get,
    path = "/api/channels/{channel_id}/messages",
    responses(
        (status = 200, description = "Message history", body = Vec<GuildMessage>),
        (status = 403, description = "Forbidden")
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
                .optional()?
                .is_some();

            if !session_exists {
                return Err(diesel::result::Error::NotFound);
            }

            let guild_id = channels::table
                .find(channel_id)
                .select(channels::guild_id.assume_not_null())
                .first::<i64>(conn)?;

            let is_member = guild_members::table
                .filter(guild_members::guild_id.eq(guild_id))
                .filter(guild_members::user_id.eq(user_id))
                .count()
                .get_result::<i64>(conn)? > 0;

            if !is_member {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            guild_messages::table
                .filter(guild_messages::channel_id.eq(channel_id))
                .order(guild_messages::created_at.desc())
                .limit(50)
                .select(GuildMessage::as_select())
                .load::<GuildMessage>(conn)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => ApiError::Unauthorized("Invalid session".into()),
            diesel::result::Error::RollbackTransaction => Forbidden("Not a member of this guild".into()),
            _ => ApiError::Internal(e.to_string()),
        })?;

    Ok(Json(messages))
}