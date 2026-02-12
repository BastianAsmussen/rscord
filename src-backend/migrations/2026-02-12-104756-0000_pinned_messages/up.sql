CREATE TABLE "pinned_messages"
(
    "id"         SERIAL PRIMARY KEY,
    "channel_id" BIGINT NOT NULL references channels (id),
    "message_id" BIGINT NOT NULL references messages (id)
);