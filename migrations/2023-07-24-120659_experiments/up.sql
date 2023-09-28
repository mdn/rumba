CREATE TABLE experiments (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    active          BOOLEAN NOT NULL DEFAULT FALSE,
    config          JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(user_id)
);

ALTER TABLE users
    ADD COLUMN is_mdn_team BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN is_fox_food BOOLEAN NOT NULL DEFAULT FALSE; 

CREATE TABLE ai_help_logs (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    variant         TEXT NOT NULL,
    chat_id         UUID NOT NULL,
    message_id      INT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT now(),
    request         JSONB NOT NULL DEFAULT '{}'::jsonb,
    response        JSONB NOT NULL DEFAULT '{}'::jsonb,
    debug           BOOLEAN NOT NULL DEFAULT FALSE,
    feedback        TEXT,
    thumbs          BOOLEAN DEFAULT NULL,
    UNIQUE(chat_id, message_id)
);