// @generated automatically by Diesel CLI.

diesel::table! {
    channels (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
    }
}

diesel::table! {
    guild_channels (id) {
        id -> Int4,
        guild_id -> Int8,
        channel_id -> Int8,
        permissions -> Int4,
    }
}

diesel::table! {
    guilds (id) {
        id -> Int4,
        owner_id -> Int8,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 255]
        icon_url -> Nullable<Varchar>,
    }
}

diesel::table! {
    guilds_users (id) {
        id -> Int4,
        guild_id -> Int8,
        user_id -> Int8,
    }
}

diesel::table! {
    messages (id) {
        id -> Int4,
        author_id -> Int8,
        reply_to_id -> Nullable<Int8>,
        channel_id -> Int8,
        #[max_length = 2000]
        contents -> Varchar,
        edited_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    pinned_messages (id) {
        id -> Int4,
        channel_id -> Int8,
        message_id -> Int8,
    }
}

diesel::table! {
    relationships (primary_key) {
        primary_key -> Int4,
        sender_id -> Int8,
        receiver_id -> Int8,
        sent_at -> Timestamp,
        accepted_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    roles (id) {
        id -> Int4,
        guild_id -> Int8,
        priority -> Int4,
        #[max_length = 32]
        name -> Varchar,
        color -> Int8,
        permissions -> Int2,
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
    user_channels (id) {
        id -> Int4,
        relationship_id -> Int8,
        channel_id -> Int8,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 320]
        email -> Varchar,
        #[max_length = 255]
        password_digest -> Bpchar,
        #[max_length = 32]
        user_handle -> Varchar,
        #[max_length = 32]
        display_name -> Varchar,
        #[max_length = 255]
        status -> Varchar,
        #[max_length = 255]
        icon_url -> Nullable<Varchar>,
        settings -> Jsonb,
        email_verified -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users_roles (id) {
        id -> Int4,
        user_id -> Int8,
        role_id -> Int8,
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

diesel::joinable!(guild_channels -> channels (channel_id));
diesel::joinable!(guild_channels -> guilds (guild_id));
diesel::joinable!(guilds -> users (owner_id));
diesel::joinable!(guilds_users -> guilds (guild_id));
diesel::joinable!(guilds_users -> users (user_id));
diesel::joinable!(messages -> channels (channel_id));
diesel::joinable!(messages -> guilds (author_id));
diesel::joinable!(pinned_messages -> channels (channel_id));
diesel::joinable!(pinned_messages -> messages (message_id));
diesel::joinable!(roles -> guilds (guild_id));
diesel::joinable!(sessions -> users (user_id));
diesel::joinable!(user_channels -> channels (channel_id));
diesel::joinable!(user_channels -> relationships (relationship_id));
diesel::joinable!(users_roles -> guilds (user_id));
diesel::joinable!(users_roles -> users (role_id));
diesel::joinable!(verification_codes -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    channels,
    guild_channels,
    guilds,
    guilds_users,
    messages,
    pinned_messages,
    relationships,
    roles,
    sessions,
    user_channels,
    users,
    users_roles,
    verification_codes,
);
