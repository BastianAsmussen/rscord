pub mod auth;
pub mod guilds;
pub mod messages;
pub mod push_token;
pub mod token;
pub mod ws_client;

#[cfg(dev)]
pub const BASE_URL: &str = "http://localhost:8080";
#[cfg(not(dev))]
pub const BASE_URL: &str = "http://rscord.asmussen.tech";

#[cfg(dev)]
pub const WS_URL: &str = "ws://localhost:8080";
#[cfg(not(dev))]
pub const WS_URL: &str = "ws://rscord.asmussen.tech";
