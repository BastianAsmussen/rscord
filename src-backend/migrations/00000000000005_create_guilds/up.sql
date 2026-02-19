CREATE TABLE guilds(
    id BIGSERIAL PRIMARY KEY,
    owner_id BIGINT NOT NULL REFERENCES users(id),
    name VARCHAR(64) NOT NULL,
    icon_url VARCHAR(255) NULL,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_owner FOREIGN KEY (owner_id) REFERENCES users(id)
);

CREATE INDEX idx_guilds_owner ON guilds (owner_id);
