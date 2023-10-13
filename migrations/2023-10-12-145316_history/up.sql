-- Your SQL goes here
DROP TABLE ai_help_logs;

CREATE TABLE ai_help_history (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    chat_id         UUID NOT NULL,
    message_id      UUID NOT NULL,
    parent_id       UUID DEFAULT NULL REFERENCES ai_help_history (message_id) ON DELETE CASCADE,
    created_at      TIMESTAMP NOT NULL DEFAULT now(),
    sources         JSONB NOT NULL DEFAULT '[]'::jsonb,
    request         JSONB NOT NULL DEFAULT '{}'::jsonb,
    response        JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(chat_id),
    UNIQUE(message_id)
);

CREATE TABLE ai_help_debug_logs (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    variant         TEXT NOT NULL,
    chat_id         UUID NOT NULL,
    message_id      UUID NOT NULL,
    parent_id       UUID DEFAULT NULL REFERENCES ai_help_history (message_id) ON DELETE CASCADE,
    created_at      TIMESTAMP NOT NULL DEFAULT now(),
    sources         JSONB NOT NULL DEFAULT '[]'::jsonb,
    request         JSONB NOT NULL DEFAULT '{}'::jsonb,
    response        JSONB NOT NULL DEFAULT '{}'::jsonb,
    feedback        TEXT,
    thumbs          BOOLEAN DEFAULT NULL,
    UNIQUE(chat_id),
    UNIQUE(message_id)
);

CREATE TABLE ai_help_feedback (
    id              BIGSERIAL PRIMARY KEY,
    message_id      UUID REFERENCES ai_help_history (message_id) ON DELETE CASCADE,
    feedback        TEXT,
    thumbs          BOOLEAN DEFAULT NULL
);