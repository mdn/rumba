CREATE TYPE ai_help_message_status AS ENUM (
    'success',
    'search_error',
    'ai_api_error',
    'completion_error',
    'moderation_error',
    'no_user_prompt_error',
    'token_limit_error',
    'timeout',
    'finished_too_long',
    'finished_content_filter',
    'finished_no_reason',
    'user_stopped',
    'user_timeout',
    'unknown'
);

CREATE TABLE ai_help_message_meta (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    chat_id             UUID NOT NULL,
    message_id          UUID NOT NULL,
    parent_id           UUID DEFAULT NULL,
    created_at          TIMESTAMP NOT NULL DEFAULT now(),
    search_duration     BIGINT NOT NULL,
    response_duration   BIGINT NOT NULL,
    query_len           BIGINT NOT NULL,
    context_len         BIGINT NOT NULL,
    response_len        BIGINT NOT NULL,
    model               text NOT NULL,
    status              ai_help_message_status NOT NULL DEFAULT 'unknown',
    sources             JSONB NOT NULL DEFAULT '[]'::jsonb,
    UNIQUE(message_id)
);
