CREATE TABLE users_roles
(
    id      SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL references guilds (id),
    role_id BIGINT NOT NULL references users (id)
);