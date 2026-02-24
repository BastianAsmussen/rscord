use anyhow::{Context, Result, anyhow};
use axum::Router;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use src_backend::api::users;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use rustls::crypto;
use rustls::crypto::CryptoProvider;
use src_backend::fcm;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL environment variable is not set!")?;

    let manager = deadpool_diesel::postgres::Manager::new(db_url, deadpool_diesel::Runtime::Tokio1);
    let pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .context("Failed to build database connection pool!")?;

    // Verify that the pool can actually hand out a connection before we
    // accept traffic - fail fast instead of serving 500s.
    {
        let conn = pool
            .get()
            .await
            .context("Failed to obtain a database connection from the pool!")?;

        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .map_err(|e| anyhow!("Migration task panicked: {e:?}"))?
            .map_err(|e| anyhow!("Failed to run database migrations: {e}"))?;

        tracing::info!("Database migrations applied successfully.");
    }

    // install our TLS cryptographic library used for API calls to fcm
    CryptoProvider::install_default(crypto::aws_lc_rs::default_provider());

    //Should be deleted before PR if not please flame me
    fcm::send_push_notification().await;

    let app = Router::new().merge(users::routes()).with_state(pool);

    let bind_addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {bind_addr}!"))?;

    tracing::info!("Listening on {bind_addr}...");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server exited with an error")?;

    tracing::info!("Server shut down gracefully.");

    Ok(())
}

/// Waits for a CTRL+C (SIGINT) signal to trigger a graceful shutdown.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler!");

    tracing::info!("Shutdown signal received, draining connections...");
}
