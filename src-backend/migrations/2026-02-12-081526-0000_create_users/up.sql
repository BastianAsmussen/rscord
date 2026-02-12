CREATE TABLE users(
    id SERIAL PRIMARY KEY,
    email VARCHAR(320) NOT NULL UNIQUE,
    password_digest CHAR(255) NOT NULL,
    user_handle VARCHAR(32) NOT NULL,
    display_name VARCHAR(32) NOT NULL,
    status VARCHAR(255) CHECK(status IN(
        'online',
        'dnd',
        'idle',
        'offline'
    )) NOT NULL DEFAULT 'online',
    icon_url VARCHAR(255) NULL,
    settings JSONB NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT '0',
    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL
);
