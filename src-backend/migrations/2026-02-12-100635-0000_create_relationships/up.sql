CREATE TYPE relationship_status AS ENUM(
    'pending',
    'accepted',
    'blocked'
);

CREATE TABLE relationships(
    id BIGSERIAL PRIMARY KEY,
    sender_id BIGINT NOT NULL,
    receiver_id BIGINT NOT NULL,
    status relationship_status NOT NULL DEFAULT 'pending',

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NULL,

    CONSTRAINT fk_sender FOREIGN KEY(sender_id) REFERENCES users(id),
    CONSTRAINT fk_receiver FOREIGN KEY(receiver_id) REFERENCES users(id),
    CONSTRAINT no_self_relationship CHECK(sender_id <> receiver_id),
    CONSTRAINT unique_relationship UNIQUE(sender_id, receiver_id)
);

-- Add index on receiver_id manually as the UNIQUE constraint covers sender_id.
CREATE INDEX idx_relationships_receiver
ON relationships(receiver_id);
