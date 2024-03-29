CREATE TABLE ai_help_limits (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             BIGINT REFERENCES users (id) ON DELETE CASCADE,
    latest_start        TIMESTAMP DEFAULT NULL,
    session_questions   BIGINT NOT NULL DEFAULT 0,
    total_questions     BIGINT NOT NULL DEFAULT 0,
    UNIQUE(user_id)
);
