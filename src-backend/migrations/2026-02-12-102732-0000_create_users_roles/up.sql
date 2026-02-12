CREATE TABLE users_roles(
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES guilds(id),
    role_id BIGINT NOT NULL REFERENCES users(id)
);
