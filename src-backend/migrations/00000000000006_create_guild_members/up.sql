CREATE TABLE guild_members(
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    joined_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    PRIMARY KEY (guild_id, user_id),

    CONSTRAINT fk_guild FOREIGN KEY (guild_id) REFERENCES guilds(id) ON DELETE CASCADE,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Needed for "which guilds is user X in?"
CREATE INDEX idx_guild_members_user ON guild_members(user_id);
