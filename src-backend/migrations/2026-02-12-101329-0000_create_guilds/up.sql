CREATE TABLE guilds
(
    id       SERIAL PRIMARY KEY,
    owner_id BIGINT NOT NULL references users (id),
    name     VARCHAR(64) NOT NULL,
    icon_url VARCHAR(255) NULL
);