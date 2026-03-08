CREATE TYPE channel_type AS ENUM (
    'text',
    'voice',
    'dm',
    'group_dm'
);

CREATE TABLE channels(
    id BIGSERIAL PRIMARY KEY,
    type channel_type NOT NULL,

    -- Guild channels only.
    guild_id BIGINT NULL,
    name VARCHAR(64) NULL,
    topic VARCHAR(1024) NULL,
    position INTEGER NULL,
    permission INTEGER NULL,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_guild FOREIGN KEY (guild_id) REFERENCES guilds(id) ON DELETE CASCADE,

    -- Guild channels MUST have a guild_id and name, DMs must NOT.
    CONSTRAINT guild_channel_check CHECK (
        CASE
            WHEN type IN ('dm', 'group_dm') THEN guild_id IS NULL AND name IS NULL
            ELSE guild_id IS NOT NULL AND name IS NOT NULL
        END
    )
);

CREATE INDEX idx_channels_guild ON channels(guild_id) WHERE guild_id IS NOT NULL;
