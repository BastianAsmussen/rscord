CREATE TABLE channel_members
(
    channel_id BIGINT NOT NULL,
    user_id    BIGINT NOT NULL,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    PRIMARY KEY (channel_id, user_id),

    CONSTRAINT fk_channel FOREIGN KEY (channel_id) REFERENCES guild_channels (id) ON DELETE CASCADE,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);