CREATE TYPE user_status AS ENUM(
    'online',
    'dnd',
    'idle',
    'offline'
);

CREATE TABLE users(
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(320) NOT NULL UNIQUE,
    password_digest VARCHAR(255) NOT NULL,
    user_handle VARCHAR(32) NOT NULL UNIQUE,
    status user_status NOT NULL DEFAULT 'online',
    settings JSONB NOT NULL DEFAULT '{}',
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);
