use crate::db::models::direct_messages::DirectMessage;
use crate::db::models::guild_messages::GuildMessageResponse;
use argon2::{Argon2, password_hash::rand_core::OsRng};
use axum::extract::FromRef;
use chrono::{Duration, NaiveDateTime, Utc};
use opaque_ke::{CipherSuite, Ristretto255, ServerLogin, ServerSetup, TripleDh};
use sha2::Sha512;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, broadcast};
use tracing::warn;

pub struct DefaultCipherSuite;

impl CipherSuite for DefaultCipherSuite {
    type OprfCs = Ristretto255;
    type KeyExchange = TripleDh<Ristretto255, Sha512>;
    type Ksf = Argon2<'static>;
}

/// Ephemeral state kept between register-start and register-finish.
/// The server is stateless during registration itself - we just need to
/// remember who is registering so we can write the DB row on finish.
pub struct PendingRegistration {
    pub email: String,
    pub handle: String,
    pub expires_at: NaiveDateTime,
}

/// Ephemeral state kept between login-start and login-finish.
/// The `ServerLogin` value holds the server's ephemeral private key and MUST
/// be matched to the same client that started the handshake.
pub struct PendingLogin {
    pub user_id: i64,
    pub server_login: ServerLogin<DefaultCipherSuite>,
    pub expires_at: NaiveDateTime,
}

/// Shared application state threaded through every Axum handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: deadpool_diesel::postgres::Pool,
    /// Long-lived server keypair.  Generated once at boot; never rotates.
    pub server_setup: Arc<ServerSetup<DefaultCipherSuite>>,
    /// In-flight registration handshakes (TTL: 5 min).
    pub pending_registrations: Arc<Mutex<HashMap<String, PendingRegistration>>>,
    /// In-flight login handshakes (TTL: 2 min).
    pub pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    /// The global message bus for real time guild events
    pub tx: broadcast::Sender<GuildMessageResponse>,
    /// The global message bus for real time encrypted DM events
    pub dm_tx: broadcast::Sender<DirectMessage>,
}

impl AppState {
    /// Load or generate the server setup from the `OPAQUE_SERVER_SETUP`
    /// environment variable (hex-encoded bytes).
    ///
    /// # Panics
    ///
    /// Panics if `OPAQUE_SERVER_SETUP` is set but contains invalid hex or
    /// cannot be deserialized into a valid `ServerSetup`.
    #[must_use]
    pub fn new(pool: deadpool_diesel::postgres::Pool) -> Self {
        let server_setup = std::env::var("OPAQUE_SERVER_SETUP").map_or_else(
            |_| {
                // First-run convenience: generate and print so you can persist
                // it. In production this path should panic instead.
                let setup = ServerSetup::<DefaultCipherSuite>::new(&mut OsRng);
                warn!(
                    "WARNING: no OPAQUE_SERVER_SETUP found. \
                     Set this env var to the following value to persist registrations:\n{}",
                    hex::encode(setup.serialize())
                );
                setup
            },
            |hex| {
                let bytes = hex::decode(hex).expect("OPAQUE_SERVER_SETUP is not valid hex");
                ServerSetup::<DefaultCipherSuite>::deserialize(&bytes)
                    .expect("OPAQUE_SERVER_SETUP deserialization failed")
            },
        );
        let (tx, _) = broadcast::channel(1024);
        let (dm_tx, _) = broadcast::channel(1024);

        Self {
            pool,
            server_setup: Arc::new(server_setup),
            pending_registrations: Arc::new(Mutex::new(HashMap::new())),
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            tx,
            dm_tx,
        }
    }

    pub async fn store_pending_registration(&self, id: &str, email: String, handle: String) {
        let mut map = self.pending_registrations.lock().await;
        map.insert(
            id.to_owned(),
            PendingRegistration {
                email,
                handle,
                expires_at: (Utc::now() + Duration::minutes(5)).naive_utc(),
            },
        );
    }

    pub async fn pop_pending_registration(&self, id: &str) -> Option<PendingRegistration> {
        let pending = self.pending_registrations.lock().await.remove(id)?;
        if pending.expires_at < Utc::now().naive_utc() {
            return None;
        }

        Some(pending)
    }

    pub async fn store_pending_login(
        &self,
        id: &str,
        user_id: i64,
        server_login: ServerLogin<DefaultCipherSuite>,
    ) {
        let mut map = self.pending_logins.lock().await;
        map.insert(
            id.to_owned(),
            PendingLogin {
                user_id,
                server_login,
                expires_at: (Utc::now() + Duration::minutes(2)).naive_utc(),
            },
        );
    }

    pub async fn pop_pending_login(&self, id: &str) -> Option<PendingLogin> {
        let pending = self.pending_logins.lock().await.remove(id)?;
        if pending.expires_at < Utc::now().naive_utc() {
            return None;
        }

        Some(pending)
    }
}

impl FromRef<AppState> for deadpool_diesel::postgres::Pool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
