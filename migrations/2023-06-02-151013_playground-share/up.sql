CREATE TABLE playground (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NULL REFERENCES users (id) ON DELETE SET NULL,
	gist            TEXT NOT NULL UNIQUE,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    flagged         BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX playground_gist ON playground (gist);
