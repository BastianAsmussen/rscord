CREATE TABLE push_tokens(
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    token CHAR(142) NOT NULL UNIQUE,

    CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_push_token_user on push_tokens(user_id);
