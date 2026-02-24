use fcm_service::{FcmMessage, FcmNotification, FcmService, Target};

pub async fn send_push_notifications(userid: i64, title: String, body: String, image: Option<String>){
    
}

pub async fn send_push_notification() -> Result<(), Box<dyn std::error::Error>> {

    let service = FcmService::new("fcm-service-account.json");

    let mut message = FcmMessage::new();
    let mut notification = FcmNotification::new();
    notification.set_title("Hello".to_string());
    notification.set_body("World".to_string());
    notification.set_image(Some("https://cdn.discordapp.com/attachments/956169411029532712/1474026132985614519/IMG_2453_optimized_1000.png?ex=69985950&is=699707d0&hm=960997b5e9279ef3ecf998456e05d4903ceb51e46b02229bf9069e0bcc2b2af1".to_string()));
    message.set_notification(Some(notification));
    message.set_target(Target::Token("ek5ze6Q3TKOKknAAIPmBVN:APA91bH6PBuKRTGiQv8SQ_kA1IOlm0mPuJ35Ousnea6SQZV6Zo_BCqfdeECljx0ewmh_Q5XkLp52ioIjwRJgsro97I3cYr4pe4wU1WG8HJlbawKg6tPOz9Q".to_string()));

    service.send_notification(message).await?;
    Ok(())
}
