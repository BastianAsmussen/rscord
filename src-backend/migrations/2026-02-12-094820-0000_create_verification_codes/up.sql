CREATE TABLE "verification_codes"
(
    id         SERIAL  PRIMARY KEY,
    user_id    BIGINT  NOT NULL references users (id),
    code       INTEGER NOT NULL,
    expires_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() + INTERVAL '15 minutes')
);