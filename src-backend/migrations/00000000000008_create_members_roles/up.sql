CREATE TABLE members_roles(
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    assigned_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    PRIMARY KEY (guild_id, user_id, role_id),

    CONSTRAINT fk_guild FOREIGN KEY (guild_id) REFERENCES guilds(id) ON DELETE CASCADE,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_role FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    CONSTRAINT fk_member FOREIGN KEY (guild_id, user_id) REFERENCES guild_members(guild_id, user_id) ON DELETE CASCADE
);

-- "What roles does user X have across all guilds?"
CREATE INDEX idx_members_roles_user ON members_roles(user_id);

-- "Who has role X?"
CREATE INDEX idx_members_roles_role ON members_roles(role_id);
