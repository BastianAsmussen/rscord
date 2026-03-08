CREATE TABLE guild_channels
(
    id         BIGSERIAL PRIMARY KEY,
    guild_id   BIGINT       NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    name       VARCHAR(64)  NOT NULL,
    type       channel_type NOT NULL CHECK (type IN ('text', 'voice')),
    topic      VARCHAR(1024),
    position   INTEGER      NOT NULL DEFAULT 0,
    permission INTEGER,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_guild_channels_guild_id ON guild_channels(guild_id);