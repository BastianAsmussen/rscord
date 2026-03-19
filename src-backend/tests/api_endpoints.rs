//! Mocked API endpoint integration tests.
//!
//! These tests spin up the full Axum router backed by a real PostgreSQL
//! database (with migrations applied) and exercise every major API endpoint
//! via [`tower::ServiceExt::oneshot`] - no HTTP server is started.
//!
//! Each test gets an isolated database: a fresh schema is created per test
//! function so there is no cross-test contamination.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use diesel::{RunQueryDsl, SelectableHelper, associations::HasTable, connection::SimpleConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use src_backend::api::{
    auth, direct_messages, guild_messages, guilds, keys, opaque::AppState, push_tokens, roles,
    users, websocket,
};
use src_backend::db::models::sessions::NewSession;
use src_backend::db::models::users::{NewUser, User};
use src_backend::db::schema::{
    displayed_users as displayed_users_schema, sessions as sessions_schema, users as users_schema,
};

/// The same embedded migrations used by the production server.
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

// Test infrastructure.

/// The base URL for the test database. The test-runner must have a Postgres
/// instance accessible at this URL. Each test creates its own schema inside
/// this database so tests do not interfere with each other.
fn test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://rscord_test:test_password@localhost:5432/rscord_test".to_string()
    })
}

/// Create an isolated **test database** and return a connection pool pointing
/// at it. Each test gets its own database to avoid cross-test contamination.
async fn test_pool(test_name: &str) -> deadpool_diesel::postgres::Pool {
    let base_url = test_database_url();

    // Database names must be simple identifiers.
    let db_name = format!("rscord_test_{test_name}");

    // Connect to the *template* database to create the test database.
    let manager =
        deadpool_diesel::postgres::Manager::new(&base_url, deadpool_diesel::Runtime::Tokio1);
    let setup_pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .expect("failed to build setup pool");

    let db_name_clone = db_name.clone();
    let conn = setup_pool.get().await.expect("setup connection");
    conn.interact(move |conn| {
        use diesel::connection::SimpleConnection;
        // Terminate any existing connections to the database.
        drop(conn.batch_execute(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = '{db_name_clone}';"
        )));
        drop(conn.batch_execute(&format!("DROP DATABASE IF EXISTS \"{db_name_clone}\";")));
        conn.batch_execute(&format!("CREATE DATABASE \"{db_name_clone}\";"))
            .expect("failed to create test database");
    })
    .await
    .expect("interact failed");

    // Build a pool that connects to the fresh test database.
    let test_url = base_url.rsplit_once('/').map_or_else(
        || format!("{base_url}/{db_name}"),
        |(prefix, _)| format!("{prefix}/{db_name}"),
    );

    let manager =
        deadpool_diesel::postgres::Manager::new(test_url, deadpool_diesel::Runtime::Tokio1);
    let pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .expect("failed to build test pool");

    // Run migrations.
    let conn = pool.get().await.expect("migration connection");
    conn.interact(|conn| {
        conn.run_pending_migrations(MIGRATIONS)
            .expect("migrations failed");
    })
    .await
    .expect("migration interact failed");

    pool
}

/// Build the full application [`Router`] backed by `pool`.
fn app(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(users::routes())
        .merge(guilds::routes())
        .merge(roles::routes())
        .merge(guild_messages::routes())
        .merge(direct_messages::routes())
        .merge(keys::routes())
        .merge(push_tokens::routes())
        .merge(websocket::routes())
        .with_state(state)
}

/// Create an `AppState` from a pool, auto-generating the OPAQUE server setup.
fn test_state(pool: deadpool_diesel::postgres::Pool) -> AppState {
    AppState::new(pool)
}

/// Seed a user directly via the database (bypasses OPAQUE registration).
/// Also creates the required `displayed_users` row (FK target for messages).
/// Returns the user's ID.
async fn seed_user(pool: &deadpool_diesel::postgres::Pool, email: &str, handle: &str) -> i64 {
    let conn = pool.get().await.expect("conn for seed_user");
    let new_user = NewUser {
        email: email.to_owned(),
        handle: handle.to_owned(),
        // Dummy OPAQUE record - not valid for real login but sufficient for
        // endpoints that only check session validity.
        opaque_record: vec![0u8; 192],
    };

    let handle_owned = handle.to_owned();

    let user: User = conn
        .interact(move |conn| {
            let user: User = diesel::insert_into(users_schema::dsl::users::table())
                .values(new_user)
                .returning(User::as_returning())
                .get_result(conn)
                .expect("insert user");

            // Create the corresponding displayed_users row so the FK
            // constraints on guild_messages.author_id and
            // direct_messages.author_id are satisfied.
            let escaped_handle = handle_owned.replace('\'', "''");

            conn.batch_execute(&format!(
                "INSERT INTO displayed_users (user_id, display_name) \
                 VALUES ({}, '{escaped_handle}');",
                user.id
            ))
            .expect("insert displayed_user");

            user
        })
        .await
        .expect("interact failed");

    user.id
}

/// Create a session token for `user_id` and return the raw token string.
async fn seed_session(pool: &deadpool_diesel::postgres::Pool, user_id: i64) -> String {
    let token = format!("test-token-{user_id}-{}", rand::random::<u64>());
    let expires_at = (Utc::now() + Duration::days(30)).naive_utc();

    let new_session = NewSession {
        token: token.clone(),
        user_id,
        expires_at,
    };

    let conn = pool.get().await.expect("conn for seed_session");
    conn.interact(move |conn| {
        diesel::insert_into(sessions_schema::dsl::sessions::table())
            .values(new_session)
            .execute(conn)
    })
    .await
    .expect("interact failed")
    .expect("insert session failed");

    token
}

/// Convenience: seed a user + session and return `(user_id, token)`.
async fn seed_authed_user(
    pool: &deadpool_diesel::postgres::Pool,
    email: &str,
    handle: &str,
) -> (i64, String) {
    let user_id = seed_user(pool, email, handle).await;
    let token = seed_session(pool, user_id).await;

    (user_id, token)
}

/// Build an authenticated GET request.
fn get_req(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("GET")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("request build failed")
}

/// Build an authenticated POST request with a JSON body.
fn post_json(uri: &str, token: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(body).expect("json encode")))
        .expect("request build failed")
}

/// Build an authenticated PUT request with a JSON body.
fn put_json(uri: &str, token: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("PUT")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(body).expect("json encode")))
        .expect("request build failed")
}

/// Build an authenticated DELETE request.
fn delete_req(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("DELETE")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("request build failed")
}

/// Collect the response body bytes into a JSON [`Value`].
async fn body_json(body: Body) -> Value {
    let bytes = body.collect().await.expect("body collect").to_bytes();
    serde_json::from_slice(&bytes).expect("body is valid json")
}

// Auth endpoints.

#[tokio::test]
async fn unauthenticated_request_returns_401() {
    let pool = test_pool("unauth_401").await;
    let state = test_state(pool);
    let router = app(state);

    // GET /api/users without any token -> 401.
    let req = Request::builder()
        .uri("/api/users")
        .body(Body::empty())
        .expect("req");

    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_token_returns_401() {
    let pool = test_pool("bad_token_401").await;
    let state = test_state(pool);
    let router = app(state);

    let req = get_req("/api/users", "totally-fake-token");
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_invalidates_session() {
    let pool = test_pool("logout_session").await;
    let (_, token) = seed_authed_user(&pool, "logout@test.com", "logout_user").await;

    let state = test_state(pool.clone());

    // First, verify the token works.
    {
        let req = get_req("/api/users", &token);
        let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Logout.
    {
        let req = post_json("/api/auth/logout", &token, &json!({}));
        let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // The token should now be invalid.
    {
        let req = get_req("/api/users", &token);
        let resp = app(state).oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

// User CRUD.

#[tokio::test]
async fn list_users_returns_seeded_user() {
    let pool = test_pool("list_users").await;
    let (_, token) = seed_authed_user(&pool, "alice@test.com", "alice").await;

    let state = test_state(pool);
    let router = app(state);

    let req = get_req("/api/users", &token);
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp.into_body()).await;
    let users = body.as_array().expect("array");
    assert!(!users.is_empty());
    assert_eq!(users[0]["handle"], "alice");
}

#[tokio::test]
async fn get_user_by_id() {
    let pool = test_pool("get_user_by_id").await;
    let (user_id, token) = seed_authed_user(&pool, "bob@test.com", "bob").await;

    let state = test_state(pool);
    let router = app(state);

    let req = get_req(&format!("/api/users/{user_id}"), &token);
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp.into_body()).await;
    assert_eq!(body["id"], user_id);
    assert_eq!(body["handle"], "bob");
}

#[tokio::test]
async fn get_nonexistent_user_returns_404() {
    let pool = test_pool("get_user_404").await;
    let (_, token) = seed_authed_user(&pool, "eve@test.com", "eve").await;

    let state = test_state(pool);
    let router = app(state);

    let req = get_req("/api/users/999999", &token);
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_user() {
    let pool = test_pool("update_user").await;
    let (user_id, token) = seed_authed_user(&pool, "upd@test.com", "old_handle").await;

    let state = test_state(pool);
    let router = app(state);

    let req = put_json(
        &format!("/api/users/{user_id}"),
        &token,
        &json!({ "handle": "new_handle" }),
    );
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp.into_body()).await;
    assert_eq!(body["handle"], "new_handle");
}

#[tokio::test]
async fn delete_user() {
    let pool = test_pool("delete_user").await;
    // We need a second user to perform the deletion (the actor) since the
    // deleted user's session token would become invalid once the user row is
    // removed.
    let (_, actor_token) = seed_authed_user(&pool, "admin@test.com", "admin").await;
    let (target_id, _) = seed_authed_user(&pool, "target@test.com", "target").await;

    let state = test_state(pool);

    let req = delete_req(&format!("/api/users/{target_id}"), &actor_token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify user is gone.
    let req = get_req(&format!("/api/users/{target_id}"), &actor_token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_anonymizes_display_profile() {
    let pool = test_pool("delete_user_anon").await;
    let (target_id, _) = seed_authed_user(&pool, "anon@test.com", "real_handle").await;
    let (_, actor_token) = seed_authed_user(&pool, "actor_anon@test.com", "actor_anon").await;

    let state = test_state(pool.clone());

    let req = delete_req(&format!("/api/users/{target_id}"), &actor_token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // The displayed_users row must still exist (message history preserved),
    // with user_id nulled by the FK cascade and display_name anonymized.
    let conn = pool.get().await.expect("conn");
    let (stored_user_id, stored_display_name): (Option<i64>, String) = conn
        .interact(move |conn| {
            use diesel::prelude::*;
            displayed_users_schema::table
                .filter(
                    displayed_users_schema::display_name.eq(format!("Deleted User {target_id}")),
                )
                .select((
                    displayed_users_schema::user_id,
                    displayed_users_schema::display_name,
                ))
                .first(conn)
        })
        .await
        .expect("interact")
        .expect("displayed_users row must survive user deletion");

    assert!(
        stored_user_id.is_none(),
        "user_id FK should be NULL after deletion"
    );
    assert_eq!(
        stored_display_name,
        format!("Deleted User {target_id}"),
        "display_name should be anonymized"
    );
}

// Guild lifecycle.

#[tokio::test]
async fn create_guild_returns_201() {
    let pool = test_pool("create_guild").await;
    let (_, token) = seed_authed_user(&pool, "guild@test.com", "guild_owner").await;

    let state = test_state(pool);
    let router = app(state);

    let req = post_json("/api/guilds", &token, &json!({ "name": "Test Guild" }));
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = body_json(resp.into_body()).await;
    assert_eq!(body["name"], "Test Guild");
    assert!(body["id"].as_i64().is_some());
}

#[tokio::test]
async fn list_my_guilds_returns_created_guild() {
    let pool = test_pool("list_guilds").await;
    let (_, token) = seed_authed_user(&pool, "g_list@test.com", "g_lister").await;

    let state = test_state(pool);

    // Create a guild.
    let req = post_json("/api/guilds", &token, &json!({ "name": "My Guild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List guilds.
    let req = get_req("/api/guilds", &token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp.into_body()).await;
    let guilds = body.as_array().expect("array");
    assert!(guilds.iter().any(|g| g["name"] == "My Guild"));
}

#[tokio::test]
async fn join_and_leave_guild() {
    let pool = test_pool("join_leave_guild").await;
    let (_, owner_token) = seed_authed_user(&pool, "own@test.com", "owner").await;
    let (_, joiner_token) = seed_authed_user(&pool, "join@test.com", "joiner").await;

    let state = test_state(pool);

    // Owner creates guild.
    let req = post_json("/api/guilds", &owner_token, &json!({ "name": "JoinGuild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Joiner joins.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/join"),
        &joiner_token,
        &json!({}),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Joiner leaves.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/leave"),
        &joiner_token,
        &json!({}),
    );
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_guild_owner_only() {
    let pool = test_pool("delete_guild_owner").await;
    let (_, owner_token) = seed_authed_user(&pool, "del_own@test.com", "del_owner").await;
    let (_, other_token) = seed_authed_user(&pool, "del_other@test.com", "del_other").await;

    let state = test_state(pool);

    // Create guild.
    let req = post_json("/api/guilds", &owner_token, &json!({ "name": "DelGuild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Non-owner tries to delete -> should fail (not found because ownership filter).
    let req = delete_req(&format!("/api/guilds/{guild_id}"), &other_token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_ne!(resp.status(), StatusCode::NO_CONTENT);

    // Owner deletes -> 204.
    let req = delete_req(&format!("/api/guilds/{guild_id}"), &owner_token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// Guild channels.

#[tokio::test]
async fn create_and_list_guild_channels() {
    let pool = test_pool("guild_channels").await;
    let (_, token) = seed_authed_user(&pool, "chan@test.com", "channer").await;

    let state = test_state(pool);

    // Create guild (auto-creates #general).
    let req = post_json("/api/guilds", &token, &json!({ "name": "ChanGuild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // List channels (should include #general).
    let req = get_req(&format!("/api/guilds/{guild_id}/channels"), &token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let chans = body_json(resp.into_body()).await;
    let chans = chans.as_array().expect("array");
    assert!(chans.iter().any(|c| c["name"] == "general"));

    // Create a second channel.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/channels"),
        &token,
        &json!({
            "type": "Text",
            "name": "random",
            "position": 1,
            "properties": {}
        }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List again - should have 2 channels.
    let req = get_req(&format!("/api/guilds/{guild_id}/channels"), &token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    let chans = body_json(resp.into_body()).await;
    assert_eq!(chans.as_array().expect("array").len(), 2);
}

// Guild messages.

#[tokio::test]
async fn send_and_get_guild_messages() {
    let pool = test_pool("guild_messages").await;
    let (_, token) = seed_authed_user(&pool, "msg@test.com", "messenger").await;

    let state = test_state(pool);

    // Create guild.
    let req = post_json("/api/guilds", &token, &json!({ "name": "MsgGuild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Get the #general channel.
    let req = get_req(&format!("/api/guilds/{guild_id}/channels"), &token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let chans = body_json(resp.into_body()).await;
    let channel_id = chans.as_array().expect("array")[0]["id"]
        .as_i64()
        .expect("channel id");

    // Send a message.
    let req = post_json(
        &format!("/api/channels/{channel_id}/messages"),
        &token,
        &json!({ "contents": "Hello, guild!" }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let msg = body_json(resp.into_body()).await;
    assert_eq!(msg["contents"], "Hello, guild!");

    // Retrieve messages.
    let req = get_req(&format!("/api/channels/{channel_id}/messages"), &token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let msgs = body_json(resp.into_body()).await;
    let msgs = msgs.as_array().expect("array");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["contents"], "Hello, guild!");
}

#[tokio::test]
async fn non_member_cannot_send_guild_message() {
    let pool = test_pool("msg_non_member").await;
    let (_, owner_token) = seed_authed_user(&pool, "own_msg@test.com", "own_msg").await;
    let (_, outsider_token) = seed_authed_user(&pool, "out_msg@test.com", "out_msg").await;

    let state = test_state(pool);

    // Owner creates guild.
    let req = post_json(
        "/api/guilds",
        &owner_token,
        &json!({ "name": "PrivateGuild" }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Get the channel.
    let req = get_req(&format!("/api/guilds/{guild_id}/channels"), &owner_token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let chans = body_json(resp.into_body()).await;
    let channel_id = chans.as_array().expect("array")[0]["id"]
        .as_i64()
        .expect("channel id");

    // Outsider tries to send a message -> forbidden.
    let req = post_json(
        &format!("/api/channels/{channel_id}/messages"),
        &outsider_token,
        &json!({ "contents": "I shouldn't be here" }),
    );
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// Roles.

#[tokio::test]
async fn role_crud_lifecycle() {
    let pool = test_pool("role_crud").await;
    let (_, owner_token) = seed_authed_user(&pool, "role_own@test.com", "role_owner").await;

    let state = test_state(pool);

    // Create guild.
    let req = post_json("/api/guilds", &owner_token, &json!({ "name": "RoleGuild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Create role.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/roles"),
        &owner_token,
        &json!({
            "name": "Moderator",
            "color": 0x00_FF_00,
            "priority": 10,
            "permissions": 0xFF
        }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let role = body_json(resp.into_body()).await;
    let role_id = role["id"].as_i64().expect("role id");
    assert_eq!(role["name"], "Moderator");

    // List roles.
    let req = get_req(&format!("/api/guilds/{guild_id}/roles"), &owner_token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let roles = body_json(resp.into_body()).await;
    assert!(!roles.as_array().expect("array").is_empty());

    // Update role.
    let req = put_json(
        &format!("/api/guilds/{guild_id}/roles/{role_id}"),
        &owner_token,
        &json!({ "name": "Admin" }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let updated = body_json(resp.into_body()).await;
    assert_eq!(updated["name"], "Admin");

    // Delete role.
    let req = delete_req(
        &format!("/api/guilds/{guild_id}/roles/{role_id}"),
        &owner_token,
    );
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn non_owner_cannot_create_role() {
    let pool = test_pool("role_non_owner").await;
    let (_, owner_token) = seed_authed_user(&pool, "r_own@test.com", "r_own").await;
    let (_, member_token) = seed_authed_user(&pool, "r_mem@test.com", "r_mem").await;

    let state = test_state(pool);

    // Create guild.
    let req = post_json(
        "/api/guilds",
        &owner_token,
        &json!({ "name": "OwnerOnlyRoles" }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Member joins guild.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/join"),
        &member_token,
        &json!({}),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Member tries to create role -> forbidden.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/roles"),
        &member_token,
        &json!({
            "name": "Hacker",
            "color": 0,
            "priority": 1,
            "permissions": 0
        }),
    );
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// Encryption key management.

#[tokio::test]
async fn upload_and_fetch_key_bundle() {
    let pool = test_pool("key_bundle").await;
    let (user_id, token) = seed_authed_user(&pool, "keys@test.com", "key_user").await;
    let (_, requester_token) = seed_authed_user(&pool, "req@test.com", "requester").await;

    let state = test_state(pool);

    // Upload identity key (32 random bytes, hex-encoded).
    let identity_key = hex::encode([0xAA; 32]);
    let req = post_json(
        "/api/keys/identity",
        &token,
        &json!({ "public_key": identity_key }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Upload signed prekey.
    let signed_prekey = hex::encode([0xBB; 32]);
    let signature = hex::encode([0xCC; 64]);
    let req = post_json(
        "/api/keys/signed-prekey",
        &token,
        &json!({ "public_key": signed_prekey, "signature": signature }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Upload one-time prekeys.
    let otpk1 = hex::encode([0xDD; 32]);
    let otpk2 = hex::encode([0xEE; 32]);
    let req = post_json(
        "/api/keys/prekeys",
        &token,
        &json!({ "prekeys": [otpk1, otpk2] }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Fetch bundle - should include identity, signed prekey, and one OTP key.
    let req = get_req(&format!("/api/keys/bundle/{user_id}"), &requester_token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let bundle = body_json(resp.into_body()).await;
    assert_eq!(bundle["identity_key"], identity_key);
    assert_eq!(bundle["signed_prekey"], signed_prekey);
    assert_eq!(bundle["signed_prekey_signature"], signature);
    // First OTP key consumed.
    assert!(bundle["one_time_prekey"].is_string());

    // Fetch again - should get the second OTP key.
    let req = get_req(&format!("/api/keys/bundle/{user_id}"), &requester_token);
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let bundle2 = body_json(resp.into_body()).await;
    assert!(bundle2["one_time_prekey"].is_string());
    // Different key than the first fetch.
    assert_ne!(
        bundle["one_time_prekey"].as_str(),
        bundle2["one_time_prekey"].as_str()
    );

    // Third fetch - no more OTP keys.
    let req = get_req(&format!("/api/keys/bundle/{user_id}"), &requester_token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let bundle3 = body_json(resp.into_body()).await;
    assert!(bundle3["one_time_prekey"].is_null());
}

#[tokio::test]
async fn identity_key_rejects_wrong_length() {
    let pool = test_pool("key_bad_len").await;
    let (_, token) = seed_authed_user(&pool, "badkey@test.com", "bad_key").await;

    let state = test_state(pool);
    let router = app(state);

    // 16 bytes instead of 32.
    let short_key = hex::encode([0xAA; 16]);
    let req = post_json(
        "/api/keys/identity",
        &token,
        &json!({ "public_key": short_key }),
    );
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// DM endpoints (encrypted).

/// Helper: Create a DM channel and add two members.
async fn setup_dm_channel(pool: &deadpool_diesel::postgres::Pool, user_a: i64, user_b: i64) -> i64 {
    let conn = pool.get().await.expect("conn");
    conn.interact(move |conn| {
        use diesel::connection::SimpleConnection;
        use diesel::dsl::sql;
        use diesel::sql_types::BigInt;

        conn.batch_execute(&format!(
            "DO $$ DECLARE ch_id BIGINT; BEGIN \
               INSERT INTO channels (type, position, properties) \
                 VALUES ('dm', 0, '{{}}') RETURNING id INTO ch_id; \
               INSERT INTO channels_members (channel_id, user_id) \
                 VALUES (ch_id, {user_a}); \
               INSERT INTO channels_members (channel_id, user_id) \
                 VALUES (ch_id, {user_b}); \
             END $$;"
        ))
        .expect("setup dm channel");

        diesel::select(sql::<BigInt>(
            "(SELECT id FROM channels WHERE type = 'dm' ORDER BY id DESC LIMIT 1)",
        ))
        .get_result::<i64>(conn)
        .expect("get channel id")
    })
    .await
    .expect("interact")
}

#[tokio::test]
async fn send_and_receive_encrypted_dm() {
    let pool = test_pool("dm_send_recv").await;
    let (alice_id, alice_token) = seed_authed_user(&pool, "alice_dm@test.com", "alice_dm").await;
    let (bob_id, bob_token) = seed_authed_user(&pool, "bob_dm@test.com", "bob_dm").await;

    let channel_id = setup_dm_channel(&pool, alice_id, bob_id).await;

    let state = test_state(pool);

    // Alice sends an encrypted message.
    let ciphertext = hex::encode(b"encrypted-payload-bytes!!");
    let nonce = hex::encode([0x42; 12]);
    let req = post_json(
        &format!("/api/dm/{channel_id}/messages"),
        &alice_token,
        &json!({
            "ciphertext": ciphertext,
            "nonce": nonce,
            "ratchet_key_id": 1
        }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let msg = body_json(resp.into_body()).await;
    assert_eq!(msg["ratchet_key_id"], 1);

    // Bob fetches messages.
    let req = get_req(&format!("/api/dm/{channel_id}/messages"), &bob_token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let msgs = body_json(resp.into_body()).await;
    let msgs = msgs.as_array().expect("array");
    assert_eq!(msgs.len(), 1);
}

#[tokio::test]
async fn dm_rejects_invalid_nonce_length() {
    let pool = test_pool("dm_bad_nonce").await;
    let (alice_id, alice_token) = seed_authed_user(&pool, "anonce@test.com", "anonce").await;
    let (bob_id, _) = seed_authed_user(&pool, "bnonce@test.com", "bnonce").await;

    let channel_id = setup_dm_channel(&pool, alice_id, bob_id).await;

    let state = test_state(pool);
    let router = app(state);

    // 8-byte nonce instead of 12.
    let bad_nonce = hex::encode([0x42; 8]);
    let req = post_json(
        &format!("/api/dm/{channel_id}/messages"),
        &alice_token,
        &json!({
            "ciphertext": hex::encode(b"test"),
            "nonce": bad_nonce,
            "ratchet_key_id": 1
        }),
    );
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// Push tokens

#[tokio::test]
async fn push_token_add_and_remove() {
    let pool = test_pool("push_tokens").await;
    let (user_id, _) = seed_authed_user(&pool, "push@test.com", "pusher").await;

    let state = test_state(pool);

    // Add push token.
    let req = Request::builder()
        .uri("/api/push-token")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "user_id": user_id,
                "token": "test-fcm-token-0123456789abcdef"
            }))
            .expect("json"),
        ))
        .expect("req");
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Remove push token.
    let req = Request::builder()
        .uri("/api/push-token/test-fcm-token-0123456789abcdef")
        .method("DELETE")
        .body(Body::empty())
        .expect("req");
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Removing again -> 404.
    let req = Request::builder()
        .uri("/api/push-token/test-fcm-token-0123456789abcdef")
        .method("DELETE")
        .body(Body::empty())
        .expect("req");
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// Guild members.

#[tokio::test]
async fn get_guild_members_lists_all_members() {
    let pool = test_pool("guild_members").await;
    let (_, owner_token) = seed_authed_user(&pool, "gm_own@test.com", "gm_owner").await;
    let (_, member_token) = seed_authed_user(&pool, "gm_mem@test.com", "gm_member").await;

    let state = test_state(pool);

    // Create guild.
    let req = post_json(
        "/api/guilds",
        &owner_token,
        &json!({ "name": "MemberGuild" }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Member joins.
    let req = post_json(
        &format!("/api/guilds/{guild_id}/join"),
        &member_token,
        &json!({}),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Get members.
    let req = get_req(&format!("/api/guilds/{guild_id}/members"), &owner_token);
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let members = body_json(resp.into_body()).await;
    let members = members.as_array().expect("array");
    assert_eq!(members.len(), 2);
}

// Guild update.

#[tokio::test]
async fn update_guild_owner_only() {
    let pool = test_pool("guild_update").await;
    let (_, owner_token) = seed_authed_user(&pool, "gu_own@test.com", "gu_owner").await;
    let (_, other_token) = seed_authed_user(&pool, "gu_other@test.com", "gu_other").await;

    let state = test_state(pool);

    // Create guild.
    let req = post_json("/api/guilds", &owner_token, &json!({ "name": "UpdGuild" }));
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    let guild = body_json(resp.into_body()).await;
    let guild_id = guild["id"].as_i64().expect("guild id");

    // Non-owner tries to update -> should fail.
    let req = put_json(
        &format!("/api/guilds/{guild_id}"),
        &other_token,
        &json!({ "name": "Hacked" }),
    );
    let resp = app(state.clone()).oneshot(req).await.expect("oneshot");
    assert_ne!(resp.status(), StatusCode::OK);

    // Owner updates -> 200.
    let req = put_json(
        &format!("/api/guilds/{guild_id}"),
        &owner_token,
        &json!({ "name": "UpdatedGuild" }),
    );
    let resp = app(state).oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp.into_body()).await;
    assert_eq!(body["name"], "UpdatedGuild");
}

// Error handling edge cases.

#[tokio::test]
async fn invalid_json_body_returns_422_or_400() {
    let pool = test_pool("bad_json").await;
    let (_, token) = seed_authed_user(&pool, "bad@test.com", "bad_json").await;

    let state = test_state(pool);
    let router = app(state);

    // Send malformed JSON to POST /api/guilds.
    let req = Request::builder()
        .uri("/api/guilds")
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(b"not-json".to_vec()))
        .expect("req");

    let resp = router.oneshot(req).await.expect("oneshot");
    // Axum returns 400 for JSON parse errors.
    let status = resp.status();
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 400 or 422, got {status}"
    );
}

#[tokio::test]
async fn missing_bundle_returns_404() {
    let pool = test_pool("bundle_404").await;
    let (_, token) = seed_authed_user(&pool, "no_bundle@test.com", "no_bundle").await;

    let state = test_state(pool);
    let router = app(state);

    // Fetch bundle for a user who never uploaded keys.
    let req = get_req("/api/keys/bundle/999999", &token);
    let resp = router.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
