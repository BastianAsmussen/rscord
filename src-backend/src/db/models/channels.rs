use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, DbEnum, ToSchema)]
#[ExistingTypePath = "crate::db::schema::sql_types::ChannelType"]
pub enum ChannelType {
    Text,
    Voice,
    Dm,
    GroupDm,
}