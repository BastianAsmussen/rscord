use crate::db::schema::private_channels;
use crate::db::models::channels::ChannelType;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = private_channels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PrivateChannel {
    pub id: i64,
    pub type_: ChannelType,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = private_channels)]
pub struct NewPrivateChannel {
    #[serde(rename = "type")]
    pub type_: ChannelType,
}