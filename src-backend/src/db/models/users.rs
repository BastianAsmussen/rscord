use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i64,

    pub email: String,
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub password_digest: String,

    #[diesel(column_name = user_handle)]
    pub handle: String,
    #[schema(value_type = Object)]
    pub settings: serde_json::Value,
    pub email_verified: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser {
    pub email: String,
    pub password_digest: String,

    #[diesel(column_name = user_handle)]
    pub handle: String,
}

#[derive(Debug, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = crate::db::schema::users)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub password_digest: Option<String>,

    #[diesel(column_name = user_handle)]
    pub handle: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub settings: Option<serde_json::Value>,
    pub email_verified: Option<bool>,
}
