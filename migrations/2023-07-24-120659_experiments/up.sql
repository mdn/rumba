CREATE TABLE experiments (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
    active          BOOLEAN NOT NULL DEFAULT FALSE,
    config          JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(user_id)
);

--CREATE TABLE AI_HELP_LOGS (
--    id              BIGSERIAL PRIMARY KEY,
--    user_id         BIGSERIAL REFERENCES users (id) ON DELETE CASCADE,
--    created_at      TIMESTAMP NOT NULL DEFAULT now(),
--    version         BIGINT NOT NULL DEFAULT 1,
--    conversation    JSONB NOT NULL DEFAULT '{}'::jsonb,
--)