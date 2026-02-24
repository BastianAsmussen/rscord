CREATE TABLE push_tokens(
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    token char(142),

    CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id),
    CONSTRAINT unique_token UNIQUE(token)
);

CREATE INDEX idx_push_token_user on push_tokens(user_id);