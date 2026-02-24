use axum::{Router, routing::get};
use src_backend::{establish_connection, fcm};
use rustls::crypto;
use rustls::crypto::CryptoProvider;

#[tokio::main]
async fn main() {
    // install our TLS cryptographic library used for API calls to fcm
    CryptoProvider::install_default(crypto::aws_lc_rs::default_provider());

    //Should be deleted before PR if not please flame me
    fcm::send_push_notification().await;

    establish_connection();
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
