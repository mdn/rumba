-- Your SQL goes here
DROP TABLE ai_help_logs;

CREATE TABLE ai_help_history (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    chat_id         UUID NOT NULL,
    label           TEXT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT now(),
    updated_at      TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE(chat_id)
);

CREATE TABLE ai_help_history_messages (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    chat_id         UUID NOT NULL REFERENCES ai_help_history (chat_id) ON DELETE CASCADE,
    message_id      UUID NOT NULL,
    parent_id       UUID DEFAULT NULL REFERENCES ai_help_history_messages (message_id) ON DELETE CASCADE,
    created_at      TIMESTAMP NOT NULL DEFAULT now(),
    sources         JSONB NOT NULL DEFAULT '[]'::jsonb,
    request         JSONB NOT NULL DEFAULT '{}'::jsonb,
    response        JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(chat_id, message_id),
    UNIQUE(message_id)
);
