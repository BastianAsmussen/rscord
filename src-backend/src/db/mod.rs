use std::env;

use diesel::{Connection, ConnectionError, PgConnection};
use dotenvy::dotenv;

pub mod models;
pub mod schema;

/// Establish a connection to the database.
///
/// # Errors
/// * If the connection string is invalid.
///
/// # Panics
/// * If `DATABASE_URL` is not defined in the environment.
pub fn establish_connection() -> Result<PgConnection, ConnectionError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgConnection::establish(&database_url)
}
