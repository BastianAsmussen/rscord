use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = crate::db::schema::push_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PushToken{
    id: i64,

    user_id: i64,
    token: String
}

impl PushToken {
    pub fn user_id(&self) -> &i64 {
        &self.user_id
    }
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::db::schema::push_tokens)]
pub struct NewPushToken{
    pub user_id: i64,
    pub token: String
}
