CREATE TABLE signed_prekeys
(
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL,
    public_key BYTEA  NOT NULL,
    signature  BYTEA  NOT NULL,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_signed_prekeys_user_id ON signed_prekeys (user_id);
