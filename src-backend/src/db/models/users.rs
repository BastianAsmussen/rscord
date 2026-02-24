use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    id: i64,

    email: String,
    password_digest: String,
    #[diesel(column_name = user_handle)]
    handle: String,

    settings: serde_json::Value,
    email_verified: bool,

    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser {
    pub email: String,
    pub password_digest: String,
    #[diesel(column_name = user_handle)]
    pub handle: String,
    pub settings: serde_json::Value,
    pub email_verified: bool,
}
