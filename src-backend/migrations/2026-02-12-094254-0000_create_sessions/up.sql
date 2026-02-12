CREATE TABLE sessions
(
    id             SERIAL PRIMARY KEY,
    token          VARCHAR(255) NOT NULL,
    user_id        BIGINT       NOT NULL references users (id),
    last_logged_in TIMESTAMP(0) WITHOUT TIME ZONE NULL,
    created_at     TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at     TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL
);