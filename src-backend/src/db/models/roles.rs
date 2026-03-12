use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::db::schema::{roles, members_roles};

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, Insertable, ToSchema)]
#[diesel(table_name = roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Role {
    pub id: i64,
    pub guild_id: i64,
    pub priority: i32,
    pub name: String,
    pub color: i32,
    pub permissions: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = roles)]
pub struct NewRole {
    #[serde(skip_deserializing)]
    pub guild_id: i64,
    pub name: String,
    pub color: i32,
    pub priority: i32,
    pub permissions: i64,
}

#[derive(Debug, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = roles)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub color: Option<i32>,
    pub priority: Option<i32>,
    pub permissions: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, Insertable, ToSchema)]
#[diesel(table_name = members_roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MemberRole {
    pub guild_id: i64,
    pub user_id: i64,
    pub role_id: i64,
    pub assigned_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = members_roles)]
pub struct AssignRole {
    pub guild_id: i64,
    pub user_id: i64,
    pub role_id: i64,
}

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = crate::db::schema::roles)]
pub struct RoleSummary {
    pub id: i64,
    pub name: String,
    pub color: i32,
    pub priority: i32,
}