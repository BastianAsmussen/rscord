use anyhow::{Context, Result, anyhow};
use axum::Router;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rustls::crypto;
use rustls::crypto::CryptoProvider;
use src_backend::api::{
    auth, direct_messages, guild_messages, guilds, keys, opaque::AppState, push_tokens,
    relationships, roles, users, websocket,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::register_start,
        auth::register_finish,
        auth::login_start,
        auth::login_finish,
        auth::logout,

        guild_messages::send_guild_message,
        guild_messages::get_guild_messages,

        direct_messages::send_direct_message,
        direct_messages::get_direct_messages,

        keys::upload_identity_key,
        keys::upload_signed_prekey,
        keys::upload_one_time_prekeys,
        keys::get_prekey_bundle,

        users::create_user,
        users::list_users,
        users::get_user,
        users::update_user,
        users::delete_user,

        guilds::create_guild,
        guilds::list_my_guilds,
        guilds::delete_guild,
        guilds::join_guild,
        guilds::leave_guild,
        guilds::create_guild_channel,
        guilds::get_guild_channels,
        guilds::get_guild_members,

        roles::create_role,
        roles::list_roles,
        roles::update_role,
        roles::delete_role,

        push_tokens::add_push_token,
        push_tokens::remove_push_token,

        relationships::create_relationship,
        relationships::get_relationships,
        relationships::update_relationship,
        relationships::delete_relationship,
    ),
    components(schemas(
        src_backend::db::models::users::User,
        src_backend::db::models::users::NewUser,
        src_backend::db::models::users::UpdateUser,
        src_backend::db::models::sessions::Session,

        src_backend::db::models::guild_messages::GuildMessage,
        src_backend::db::models::guild_messages::NewGuildMessage,

        src_backend::db::models::direct_messages::DirectMessage,
        src_backend::db::models::direct_messages::NewDirectMessage,

        src_backend::db::models::keys::IdentityKey,
        src_backend::db::models::keys::UploadIdentityKey,
        src_backend::db::models::keys::SignedPrekey,
        src_backend::db::models::keys::UploadSignedPrekey,
        src_backend::db::models::keys::OneTimePrekey,
        src_backend::db::models::keys::UploadOneTimePrekeys,
        src_backend::db::models::keys::PreKeyBundleResponse,

        src_backend::db::models::guilds::Guild,
        src_backend::db::models::guilds::NewGuild,
        src_backend::db::models::guilds::GuildSummary,
        src_backend::db::models::guilds::GuildMemberWithRoles,

        src_backend::db::models::roles::Role,
        src_backend::db::models::roles::NewRole,
        src_backend::db::models::roles::UpdateRole,
        src_backend::db::models::roles::RoleSummary,

        src_backend::db::models::channels::ChannelType,
        src_backend::db::models::channels::Channel,
        src_backend::db::models::channels::NewChannel,
        src_backend::db::models::channels::UpdateChannel,

        src_backend::db::models::push_tokens::NewPushToken,

        auth::AuthResponse,
        auth::OpaqueRegisterStartRequest,
        auth::OpaqueRegisterStartResponse,
        auth::OpaqueRegisterFinishRequest,
        auth::OpaqueLoginStartRequest,
        auth::OpaqueLoginStartResponse,
        auth::OpaqueLoginFinishRequest,

        src_backend::api::errors::ErrorBody,


        src_backend::db::models::relationships::Relationship,
        src_backend::db::models::relationships::NewRelationship,
        src_backend::db::models::relationships::UpdateRelationship,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User endpoints"),
        (name = "guilds", description = "Guild endpoints"),
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

    // Install our TLS cryptographic library used for API calls to FCM.
    CryptoProvider::install_default(crypto::aws_lc_rs::default_provider())
        .map_err(|e| anyhow!("Failed to get provider for TLS: {:?}", *e))?;

    // Build AppState from the pool - this loads/generates the OPAQUE server keypair.
    let state = AppState::new(pool);

    let app = Router::new()
        .merge(auth::routes())
        .merge(users::routes())
        .merge(guilds::routes())
        .merge(roles::routes())
        .merge(guild_messages::routes())
        .merge(direct_messages::routes())
        .merge(keys::routes())
        .merge(push_tokens::routes())
        .merge(websocket::routes())
        .merge(relationships::routes())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state);

    let bind_addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {bind_addr}!"))?;

    tracing::info!("Listening on {bind_addr}...");
    tracing::info!("Swagger UI available at http://{bind_addr}/swagger-ui/");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server exited with an error!")?;

    tracing::info!("Server shut down gracefully.");

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler!");

    tracing::info!("Shutdown signal received, draining connections...");
}
