CREATE TABLE direct_messages(
    id BIGSERIAL PRIMARY KEY,
    author_id BIGINT NOT NULL,
    reply_to_id BIGINT NULL,
    channel_id BIGINT NOT NULL,
    ciphertext BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    ratchet_key_id BIGINT NOT NULL,

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_author FOREIGN KEY (author_id) REFERENCES users(id),
    CONSTRAINT fk_reply_to FOREIGN KEY (reply_to_id) REFERENCES direct_messages(id),
    CONSTRAINT fk_channel FOREIGN KEY (channel_id) REFERENCES channels(id)
);
