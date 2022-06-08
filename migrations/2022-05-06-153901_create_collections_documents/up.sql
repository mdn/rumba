CREATE TABLE documents
(
    id           BIGSERIAL PRIMARY KEY,
    created_at   TIMESTAMP NOT NULL DEFAULT now(),
    updated_at   TIMESTAMP NOT NULL DEFAULT now(),
    absolute_uri TEXT      NOT NULL UNIQUE,
    uri          TEXT      NOT NULL UNIQUE,
    metadata     JSONB,
    title        TEXT NOT NULL,
    paths        TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[]
);

CREATE TABLE collections
(
    id          BIGSERIAL PRIMARY KEY,
    created_at  TIMESTAMP NOT NULL DEFAULT now(),
    updated_at  TIMESTAMP NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMP,
    document_id BIGSERIAL references documents (id),
    notes       TEXT,
    custom_name TEXT,
    user_id     BIGSERIAL REFERENCES users (id),
    UNIQUE(document_id, user_id)
);

CREATE INDEX idx_document_paths ON documents USING GIN(paths);
