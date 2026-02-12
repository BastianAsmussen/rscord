CREATE TABLE messages
(
    id          SERIAL        PRIMARY KEY,
    author_id   BIGINT        NOT NULL references guilds (id),
    reply_to_id BIGINT        NULL references messages (id),
    channel_id  BIGINT        NOT NULL references channels (id),
    contents    VARCHAR(2000) NOT NULL,
    edited_at   TIMESTAMP(0)  WITHOUT TIME ZONE NULL,
    created_at  TIMESTAMP(0)  WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);