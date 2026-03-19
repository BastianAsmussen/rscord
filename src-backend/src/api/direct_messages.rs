use crate::api::auth_extractor::AuthUser;
use crate::api::errors::ApiError;
use crate::api::opaque::AppState;
use crate::db::models::direct_messages::{DirectMessage, NewDirectMessage};
use crate::db::schema::{channels, channels_members, direct_messages, displayed_users};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use diesel::prelude::*;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/dm/{channel_id}/messages", post(send_direct_message))
        .route("/api/dm/{channel_id}/messages", get(get_direct_messages))
}

/// Send an encrypted direct message.
///
/// The ciphertext and nonce are produced client-side using the Double Ratchet
/// protocol. The server stores them opaquely - it cannot decrypt the content.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::Forbidden`: If the user is not a member of this DM channel.
/// - `ApiError::UnprocessableEntity`: If the payload is malformed.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/dm/{channel_id}/messages",
    request_body = NewDirectMessage,
    responses(
        (status = 201, description = "Encrypted DM sent", body = DirectMessage),
        (status = 403, description = "Not a member of this DM channel")
    ),
    params(("channel_id" = i64, Path, description = "DM channel ID"))
)]
pub async fn send_direct_message(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<i64>,
    Json(payload): Json<NewDirectMessage>,
) -> Result<(StatusCode, Json<DirectMessage>), ApiError> {
    let ciphertext_bytes = hex::decode(&payload.ciphertext)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid hex in ciphertext".into()))?;
    let nonce_bytes = hex::decode(&payload.nonce)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid hex in nonce".into()))?;

    if nonce_bytes.len() != 12 {
        return Err(ApiError::UnprocessableEntity(
            "nonce must be exactly 12 bytes (24 hex chars)".into(),
        ));
    }

    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;
    let ratchet_key_id = payload.ratchet_key_id;
    let reply_to_id = payload.reply_to_id;

    let message = conn
        .interact(move |conn| {
            // Verify the channel is a DM channel and the user is a member.
            let is_dm_member = channels_members::table
                .inner_join(channels::table.on(channels::id.eq(channels_members::channel_id)))
                .filter(channels_members::channel_id.eq(channel_id))
                .filter(channels_members::user_id.eq(user_id))
                .filter(
                    channels::type_
                        .eq(crate::db::models::channels::ChannelType::Dm)
                        .or(channels::type_.eq(crate::db::models::channels::ChannelType::GroupDm)),
                )
                .count()
                .get_result::<i64>(conn)?
                > 0;

            if !is_dm_member {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            // Resolve the displayed_users.id for this user (the FK target for
            // direct_messages.author_id). Created atomically at registration.
            let displayed_user_id: i64 = displayed_users::table
                .filter(displayed_users::user_id.eq(user_id))
                .select(displayed_users::id)
                .first::<i64>(conn)?;

            diesel::insert_into(direct_messages::table)
                .values((
                    direct_messages::author_id.eq(displayed_user_id),
                    direct_messages::channel_id.eq(channel_id),
                    direct_messages::reply_to_id.eq(reply_to_id),
                    direct_messages::ciphertext.eq(&ciphertext_bytes),
                    direct_messages::nonce.eq(&nonce_bytes),
                    direct_messages::ratchet_key_id.eq(ratchet_key_id),
                ))
                .returning(DirectMessage::as_returning())
                .get_result::<DirectMessage>(conn)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::RollbackTransaction => {
                ApiError::Forbidden("Not a member of this DM channel".into())
            }
            _ => ApiError::Internal(e.to_string()),
        })?;

    // Broadcast via WebSocket so the recipient receives it in real time.
    drop(state.dm_tx.send(message.clone()));
    Ok((StatusCode::CREATED, Json(message)))
}

/// Retrieve encrypted direct message history for a DM channel.
///
/// Returns the last 50 messages (newest first). Each message contains
/// ciphertext, nonce, and ratchet key ID that the client uses to decrypt.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::Forbidden`: If the user is not a member of this DM channel.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    get,
    path = "/api/dm/{channel_id}/messages",
    responses(
        (status = 200, description = "Encrypted DM history", body = Vec<DirectMessage>),
        (status = 403, description = "Not a member of this DM channel")
    ),
    params(("channel_id" = i64, Path, description = "DM channel ID"))
)]
pub async fn get_direct_messages(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(channel_id): Path<i64>,
) -> Result<Json<Vec<DirectMessage>>, ApiError> {
    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;

    let messages = conn
        .interact(move |conn| {
            // Verify the user is a member of this DM channel.
            let is_dm_member = channels_members::table
                .inner_join(channels::table.on(channels::id.eq(channels_members::channel_id)))
                .filter(channels_members::channel_id.eq(channel_id))
                .filter(channels_members::user_id.eq(user_id))
                .filter(
                    channels::type_
                        .eq(crate::db::models::channels::ChannelType::Dm)
                        .or(channels::type_.eq(crate::db::models::channels::ChannelType::GroupDm)),
                )
                .count()
                .get_result::<i64>(conn)?
                > 0;

            if !is_dm_member {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            direct_messages::table
                .filter(direct_messages::channel_id.eq(channel_id))
                .order(direct_messages::created_at.desc())
                .limit(50)
                .select(DirectMessage::as_select())
                .load::<DirectMessage>(conn)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::RollbackTransaction => {
                ApiError::Forbidden("Not a member of this DM channel".into())
            }
            _ => ApiError::Internal(e.to_string()),
        })?;

    Ok(Json(messages))
}
