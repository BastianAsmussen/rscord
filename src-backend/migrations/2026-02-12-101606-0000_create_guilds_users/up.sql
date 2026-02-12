CREATE TABLE guilds_users
(
    id SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guilds(id),
    user_id BIGINT NOT NULL REFERENCES users(id)
);
