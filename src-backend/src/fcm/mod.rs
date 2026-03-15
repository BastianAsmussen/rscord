use crate::db::{models::push_tokens::PushToken, schema::push_tokens};
use anyhow::{Context, Result};
use axum::extract::State;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use fcm_service::{FcmMessage, FcmNotification, FcmService, Target};
use tracing::{error, warn};

type Pool = deadpool_diesel::postgres::Pool;

/// Sends push notifications to all registered Android devices for a user.
///
/// If a token is found to be unregistered (stale), it is automatically removed
/// from the database.
///
/// # Errors
///
/// Returns an error if the database connection cannot be acquired or if
/// querying push tokens fails.
// TODO: Call this function when a message is sent and the user is offline.
pub async fn send_push_notifications(
    State(pool): State<Pool>,
    user_id: i64,
    title: &str,
    body: &str,
    image: Option<String>,
) -> Result<()> {
    let tokens = fetch_push_tokens(&pool, user_id).await?;

    for token in &tokens {
        if let Err(e) = send_push_notification(token.token(), title, body, image.clone()).await {
            match e {
                SendError::StaleToken => {
                    warn!("Removing stale push token");
                    remove_push_token(&pool, token.token()).await;
                }
                SendError::Transport(msg) => {
                    error!("Failed to send push notification: {msg}");
                }
            }
        }
    }

    Ok(())
}

async fn fetch_push_tokens(pool: &Pool, user_id: i64) -> Result<Vec<PushToken>> {
    let conn = pool.get().await.context("Failed to get DB connection")?;

    conn.interact(move |conn| {
        push_tokens::dsl::push_tokens
            .filter(push_tokens::dsl::user_id.eq(user_id))
            .select(PushToken::as_select())
            .load(conn)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Interaction with DB failed: {e}"))?
    .context("Failed to query push tokens")
}

#[derive(Debug)]
enum SendError {
    StaleToken,
    Transport(String),
}

async fn send_push_notification(
    token: &str,
    title: &str,
    body: &str,
    image: Option<String>,
) -> Result<(), SendError> {
    let service = FcmService::new("fcm-service-account.json");

    let mut notification = FcmNotification::new();
    notification.set_title(title.to_string());
    notification.set_body(body.to_string());
    notification.set_image(image);

    let mut message = FcmMessage::new();
    message.set_notification(Some(notification));
    message.set_target(Target::Token(token.to_string()));

    service.send_notification(message).await.map_err(|e| {
        let error_string = e.to_string();
        if error_string.contains("UNREGISTERED") {
            SendError::StaleToken
        } else {
            SendError::Transport(error_string)
        }
    })
}

async fn remove_push_token(pool: &Pool, token: &str) {
    let conn = match pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Could not connect to DB to delete push token: {e}");
            return;
        }
    };

    let token = token.to_string();
    let result = conn
        .interact(move |conn| {
            diesel::delete(push_tokens::dsl::push_tokens.filter(push_tokens::dsl::token.eq(&token)))
                .execute(conn)
        })
        .await;

    match result {
        Ok(Ok(0)) => warn!("Push token not found during deletion!"),
        Ok(Err(e)) => error!("Diesel error deleting push token: {e}"),
        Err(e) => error!("Interaction error deleting push token: {e}"),
        _ => {}
    }
}
