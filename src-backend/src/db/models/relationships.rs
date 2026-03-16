use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, DbEnum, ToSchema)]
#[ExistingTypePath = "crate::db::schema::sql_types::RelationshipStatus"]
pub enum RelationshipStatus {
    Pending,
    Accepted,
    Blocked,
}


#[derive(Debug, Serialize, Deserialize, Selectable, Queryable, ToSchema)]
#[diesel(table_name = crate::db::schema::relationships)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Relationship {
    pub id: i64,
    pub sender_id: i64,
    pub receiver_id: i64,
    pub status: RelationshipStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = crate::db::schema::relationships)]
pub struct NewRelationship {
    pub sender_id: i64,
    pub receiver_id: i64,
    pub status: RelationshipStatus,
}

#[derive(Debug, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = crate::db::schema::relationships)]
pub struct UpdateRelationship {
    pub status: RelationshipStatus,
}