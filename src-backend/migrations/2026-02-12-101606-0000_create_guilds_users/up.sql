CREATE TABLE guilds_users
(
    id       SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL references guilds (id),
    user_id  BIGINT NOT NULL references users (id)
);