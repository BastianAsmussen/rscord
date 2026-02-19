// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "channel_type"))]
    pub struct ChannelType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "relationship_status"))]
    pub struct RelationshipStatus;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "user_status"))]
    pub struct UserStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ChannelType;

    channels (id) {
        id -> Int8,
        #[sql_name = "type"]
        type_ -> ChannelType,
        guild_id -> Nullable<Int8>,
        #[max_length = 64]
        name -> Nullable<Varchar>,
        #[max_length = 1024]
        topic -> Nullable<Varchar>,
        position -> Nullable<Int4>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    direct_messages (id) {
        id -> Int8,
        author_id -> Int8,
        reply_to_id -> Nullable<Int8>,
        channel_id -> Int8,
        ciphertext -> Bytea,
        nonce -> Bytea,
        ratchet_key_id -> Int8,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::UserStatus;

    displayed_users (id) {
        id -> Int8,
        user_id -> Nullable<Int8>,
        #[max_length = 32]
        display_name -> Varchar,
        #[max_length = 255]
        icon_url -> Nullable<Varchar>,
        status -> UserStatus,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    guild_members (guild_id, user_id) {
        guild_id -> Int8,
        user_id -> Int8,
        joined_at -> Timestamp,
    }
}

diesel::table! {
    guild_messages (id) {
        id -> Int8,
        author_id -> Int8,
        reply_to_id -> Nullable<Int8>,
        channel_id -> Int8,
        #[max_length = 2000]
        contents -> Nullable<Varchar>,
        edited_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    guilds (id) {
        id -> Int8,
        owner_id -> Int8,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 255]
        icon_url -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    members_roles (guild_id, user_id, role_id) {
        guild_id -> Int8,
        user_id -> Int8,
        role_id -> Int8,
        assigned_at -> Timestamp,
    }
}

diesel::table! {
    pinned_direct_messages (channel_id, message_id) {
        channel_id -> Int8,
        message_id -> Int8,
        pinned_by -> Int8,
        pinned_at -> Timestamp,
    }
}

diesel::table! {
    pinned_guild_messages (channel_id, message_id) {
        channel_id -> Int8,
        message_id -> Int8,
        pinned_by -> Int8,
        pinned_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::RelationshipStatus;

    relationships (id) {
        id -> Int8,
        sender_id -> Int8,
        receiver_id -> Int8,
        status -> RelationshipStatus,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    roles (id) {
        id -> Int8,
        guild_id -> Int8,
        priority -> Int4,
        #[max_length = 32]
        name -> Varchar,
        color -> Int4,
        permissions -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    sessions (id) {
        id -> Int4,
        #[max_length = 255]
        token -> Varchar,
        user_id -> Int8,
        last_logged_in -> Nullable<Timestamp>,
        created_at -> Timestamp,
        expires_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        #[max_length = 320]
        email -> Varchar,
        #[max_length = 255]
        password_digest -> Varchar,
        #[max_length = 32]
        user_handle -> Varchar,
        settings -> Jsonb,
        email_verified -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    verification_codes (id) {
        id -> Int4,
        user_id -> Int8,
        code -> Int4,
        expires_at -> Timestamp,
    }
}

diesel::joinable!(channels -> guilds (guild_id));
diesel::joinable!(direct_messages -> channels (channel_id));
diesel::joinable!(direct_messages -> displayed_users (author_id));
diesel::joinable!(displayed_users -> users (user_id));
diesel::joinable!(guild_members -> guilds (guild_id));
diesel::joinable!(guild_members -> users (user_id));
diesel::joinable!(guild_messages -> channels (channel_id));
diesel::joinable!(guild_messages -> displayed_users (author_id));
diesel::joinable!(members_roles -> guilds (guild_id));
diesel::joinable!(members_roles -> roles (role_id));
diesel::joinable!(members_roles -> users (user_id));
diesel::joinable!(pinned_direct_messages -> channels (channel_id));
diesel::joinable!(pinned_direct_messages -> direct_messages (message_id));
diesel::joinable!(pinned_direct_messages -> displayed_users (pinned_by));
diesel::joinable!(pinned_guild_messages -> channels (channel_id));
diesel::joinable!(pinned_guild_messages -> guild_messages (message_id));
diesel::joinable!(pinned_guild_messages -> users (pinned_by));
diesel::joinable!(roles -> guilds (guild_id));
diesel::joinable!(sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    channels,
    direct_messages,
    displayed_users,
    guild_members,
    guild_messages,
    guilds,
    members_roles,
    pinned_direct_messages,
    pinned_guild_messages,
    relationships,
    roles,
    sessions,
    users,
    verification_codes,
);
