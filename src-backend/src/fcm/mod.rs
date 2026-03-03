use axum::extract::State;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use fcm_service::{FcmMessage, FcmNotification, FcmService, Target};
use tracing::{info, warn, error};
use crate::db::{
    models::push_tokens::PushToken,
    schema::push_tokens,
};

type Pool = deadpool_diesel::postgres::Pool;

// TODO: call the function when a massage is sent an the user is offline
/// Sends out push notification to the user if they have android push tokens
pub async fn send_push_notifications(State(pool): State<Pool>,
                                     userid: i64,
                                     title: &str,
                                     body: &str,
                                     image: Option<String>){
    let conn = pool.get().await.expect("Could not get the thread pool");
    let result_tokens = conn
        .interact(move |conn| {
            push_tokens::dsl::push_tokens.
                filter(push_tokens::dsl::user_id.eq(userid))
                .select(PushToken::as_select())
                .load(conn)
        }).await;
    if let Ok(Ok(push_tokens)) = result_tokens {
        for token in push_tokens.iter() {
            if let Err(e) = send_push_notification(token.token(), title, body, image.clone()).await{
                info!(e);
                remove_push_token(State(pool.clone()), token.token().to_string()).await;
            }
        }
    }
}

async fn send_push_notification(token: &str,
                                title: &str,
                                body: &str,
                                image: Option<String>
) -> Result<(), String> {
    let service = FcmService::new("fcm-service-account.json");

    let mut message = FcmMessage::new();
    let mut notification = FcmNotification::new();
    notification.set_title(title.to_string());
    notification.set_body(body.to_string());
    notification.set_image(image);
    message.set_notification(Some(notification));
    message.set_target(Target::Token(token.to_string()));

    if let Err(e) = service.send_notification(message).await{
        let error_string = format!("{}", &*e);
        if error_string.contains("UNREGISTERED"){
            info!("Token is no longer valid");
            return Err("DELETED".to_string())
        }
    }
    Ok(())
}

async fn remove_push_token(State(pool): State<Pool>, token: String){
    if let Ok(conn) = pool.get().await {
        let token_for_error = token.clone();
        let rows_deleted = conn
            .interact(move |conn| {
                diesel::delete(push_tokens::dsl::push_tokens.filter(push_tokens::dsl::token.eq(token)))
                    .execute(conn)
            }).await;

        if let Ok(Ok(rows_deleted)) = rows_deleted {
            if rows_deleted == 0 {
                warn!("Error deleting push token: {}", token_for_error)
            }
        }
    }
    else {
        error!("Could not connect to DB in delete_push_token function")
    }
}
