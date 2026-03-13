CREATE TYPE user_status AS ENUM(
    'online',
    'dnd', -- "Do not Disturb"
    'idle',
    'offline'
);

CREATE TABLE displayed_users(
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NULL,
    display_name VARCHAR(32) NOT NULL,
    icon_url VARCHAR(255) NULL,
    status user_status NOT NULL DEFAULT 'online',

    created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE SET NULL
);
