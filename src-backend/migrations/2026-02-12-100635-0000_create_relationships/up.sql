CREATE TABLE relationships(
    primary_key SERIAL PRIMARY KEY,
    sender_id BIGINT NOT NULL REFERENCES users(id),
    receiver_id BIGINT NOT NULL REFERENCES users(id),
    sent_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMP(0) WITHOUT TIME ZONE NULL
);

