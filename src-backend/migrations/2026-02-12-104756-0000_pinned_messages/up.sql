CREATE TABLE pinned_messages(
    id SERIAL PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id),
    message_id BIGINT NOT NULL REFERENCES messages(id)
);
