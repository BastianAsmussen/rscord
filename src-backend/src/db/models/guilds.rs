use crate::db::schema::guild_members;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, Insertable, ToSchema)]
#[diesel(table_name = crate::db::schema::guilds)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Guild {
    pub id: i64,
    pub owner_id: i64,
    pub name: String,
    pub icon_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = crate::db::schema::guilds)]
pub struct GuildSummary {
    pub id: i64,
    pub name: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = crate::db::schema::guilds)]
pub struct NewGuild {
    #[serde(skip_deserializing)]
    pub owner_id: i64,
    pub name: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = guild_members)]
pub struct NewGuildMember {
    pub guild_id: i64,
    pub user_id: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GuildMemberWithRoles {
    pub user_id: i64,
    pub user_handle: String,
    pub roles: Vec<crate::db::models::roles::RoleSummary>,
}

#[derive(Debug, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = crate::db::schema::guilds)]
pub struct UpdateGuild {
    pub name: Option<String>,
    pub icon_url: Option<String>,
}
