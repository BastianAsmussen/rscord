use crate::db::schema::channels_members;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema, Associations)]
#[diesel(belongs_to(crate::db::models::channels::Channel))]
#[diesel(table_name = channels_members)]
#[diesel(primary_key(channel_id, user_id))]
pub struct ChannelsMembers {
    pub channel_id: i64,
    pub user_id: i64,
    pub created_at: NaiveDateTime,
}
