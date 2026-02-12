CREATE TABLE roles(
    id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    name VARCHAR(32) NOT NULL,
    color INTEGER NOT NULL DEFAULT 0,
    permissions BIGINT NOT NULL DEFAULT 0,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_guild FOREIGN KEY (guild_id) REFERENCES guilds(id),
    CONSTRAINT unique_role_name_per_guild UNIQUE (guild_id, name),
    CONSTRAINT unique_priority_per_guild UNIQUE (guild_id, priority)
);
