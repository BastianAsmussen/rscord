CREATE TYPE channel_type AS ENUM ('text', 'voice', 'dm', 'group_dm');

CREATE TABLE channels
(
    id         BIGSERIAL PRIMARY KEY,
    guild_id   BIGINT,       -- NULL for DMs
    type       channel_type NOT NULL,
    name       VARCHAR(100), -- NULL for DMs
    position   INTEGER      NOT NULL DEFAULT 0,
    -- Properties (Permissions, Topic, etc.).
    properties JSONB        NOT NULL DEFAULT '{}',

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_channels_guild_id ON channels (guild_id) WHERE (guild_id IS NOT NULL);

ALTER TABLE channels
    ADD CONSTRAINT check_guild_channel_integrity CHECK (
        (type IN ('text', 'voice') AND guild_id IS NOT NULL AND name IS NOT NULL) OR
        (type IN ('dm', 'group_dm')) -- DMs/Group DMs can have NULLs here
        );