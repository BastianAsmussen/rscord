CREATE TABLE pinned_direct_messages(
    channel_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    pinned_by BIGINT NOT NULL,
    pinned_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    PRIMARY KEY (channel_id, message_id),

    CONSTRAINT fk_channel FOREIGN KEY (channel_id) REFERENCES channels(id),
    CONSTRAINT fk_message FOREIGN KEY (message_id) REFERENCES direct_messages(id),
    CONSTRAINT fk_pinned_by FOREIGN KEY (pinned_by) REFERENCES users(id)
);
