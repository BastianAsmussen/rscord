pub mod auth;
pub mod guilds;
pub mod messages;
pub mod push_token;
pub mod token;
pub mod ws_client;

// TODO: needs to be updated when we get server
static BASE_URL: &str = "http://127.0.0.1:8080";
pub const WS_URL: &str = "ws://127.0.0.1:8080";
