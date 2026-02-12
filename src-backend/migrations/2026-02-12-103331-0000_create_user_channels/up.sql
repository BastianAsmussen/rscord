CREATE TABLE user_channels
(
    id              SERIAL PRIMARY KEY,
    relationship_id BIGINT NOT NULL references relationships (primary_key),
    channel_id      BIGINT NOT NULL references channels (id)
);