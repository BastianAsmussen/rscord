CREATE TABLE guild_messages(
    id BIGSERIAL PRIMARY KEY,
    author_id BIGINT NOT NULL,
    reply_to_id BIGINT NULL,
    channel_id BIGINT NOT NULL,
    contents VARCHAR(2000) NULL,

    edited_at TIMESTAMP(0) WITHOUT TIME ZONE NULL,
    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_author FOREIGN KEY (author_id) REFERENCES displayed_users(id),
    CONSTRAINT fk_reply_to FOREIGN KEY (reply_to_id) REFERENCES guild_messages(id),
    CONSTRAINT fk_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE
);
