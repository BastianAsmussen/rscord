CREATE TYPE channel_type AS ENUM ('text', 'voice', 'dm', 'group_dm');

CREATE TABLE private_channels
(
    id         BIGSERIAL PRIMARY KEY,
    type       channel_type NOT NULL CHECK (type IN ('dm', 'group_dm')),

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);
