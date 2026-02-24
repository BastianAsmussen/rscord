use axum::{Router, routing::post};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use src_backend::{create_user, fcm};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use rustls::crypto;
use rustls::crypto::CryptoProvider;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap();

    // Set up connection pool.
    let manager = deadpool_diesel::postgres::Manager::new(db_url, deadpool_diesel::Runtime::Tokio1);
    let pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .unwrap();

    // Run the migrations on server startup.
    {
        let conn = pool.get().await.unwrap();
        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .unwrap()
            .unwrap();
    }

    // install our TLS cryptographic library used for API calls to fcm
    CryptoProvider::install_default(crypto::aws_lc_rs::default_provider());

    //Should be deleted before PR if not please flame me
    fcm::send_push_notification().await;

    // Build our application with routes and shared state.
    let app = Router::new()
        .route("/users", post(create_user))
        .with_state(pool);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}