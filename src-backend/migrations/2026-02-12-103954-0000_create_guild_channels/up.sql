CREATE TABLE guild_channels(
    id SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guilds(id),
    channel_id BIGINT NOT NULL REFERENCES channels(id),
    permissions INTEGER NOT NULL DEFAULT 0
);
