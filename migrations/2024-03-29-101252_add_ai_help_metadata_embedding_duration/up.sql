ALTER TABLE ai_help_message_meta
ADD COLUMN embedding_duration BIGINT DEFAULT NULL,
ADD COLUMN embedding_model TEXT NOT NULL DEFAULT '';
