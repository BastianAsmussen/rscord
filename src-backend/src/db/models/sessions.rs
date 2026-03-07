use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

/// A session row as read from the database.
#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = crate::db::schema::sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Session {
    pub id: i32,

    #[schema(ignore)]
    pub token: String,
    pub user_id: i64,
    pub last_logged_in: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

/// Values needed to insert a new session.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::sessions)]
pub struct NewSession {
    pub token: String,
    pub user_id: i64,
    pub expires_at: NaiveDateTime,
}
