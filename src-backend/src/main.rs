use anyhow::{Context, Result, anyhow};
use axum::Router;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use src_backend::api::{auth, users};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::register,
        auth::login,
        auth::logout,

        users::create_user,
        users::list_users,
        users::get_user,
        users::update_user,
        users::delete_user,
    ),
    components(schemas(
        src_backend::db::models::users::User,
        src_backend::db::models::users::NewUser,
        src_backend::db::models::users::UpdateUser,
        src_backend::db::models::sessions::Session,
        src_backend::api::errors::ErrorBody,
        auth::RegisterRequest,
        auth::LoginRequest,
        auth::AuthResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User endpoints"),
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "session_token",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Cookie(
                        utoipa::openapi::security::ApiKeyValue::new("session_token"),
                    ),
                ),
            );
        }
    }
}

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

    let app = Router::new()
        .merge(auth::routes())
        .merge(users::routes())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(pool);

    let bind_addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {bind_addr}!"))?;

    tracing::info!("Listening on {bind_addr}...");
    tracing::info!("Swagger UI available at http://{bind_addr}/swagger-ui/");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server exited with an error")?;

    tracing::info!("Server shut down gracefully.");

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler!");

    tracing::info!("Shutdown signal received, draining connections...");
}
