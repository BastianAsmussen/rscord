use axum::Router;
use crate::api::opaque::AppState;
use crate::api::websocket::ws_handler;

pub mod auth;
pub mod auth_extractor;
pub mod errors;
pub mod opaque;
pub mod password;
pub mod users;
pub mod guilds;
pub mod roles;
pub mod push_tokens;
pub mod messages;
pub mod websocket;