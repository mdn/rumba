CREATE TABLE documents
(
    id           BIGSERIAL PRIMARY KEY,
    created_at   TIMESTAMP NOT NULL DEFAULT now(),
    updated_at   TIMESTAMP NOT NULL DEFAULT now(),
    absolute_uri TEXT      NOT NULL UNIQUE,
    uri          TEXT      NOT NULL UNIQUE,
    metadata     JSONB,
    title        TEXT NOT NULL
);

CREATE TABLE collections
(
    id          BIGSERIAL PRIMARY KEY,
    created_at  TIMESTAMP NOT NULL DEFAULT now(),
    updated_at  TIMESTAMP NOT NULL DEFAULT now(),
    document_id BIGSERIAL references documents (id),
    notes       TEXT,
    custom_name TEXT,
    user_id     BIGSERIAL REFERENCES users (id),
    UNIQUE(document_id, user_id)
);

CREATE UNIQUE INDEX document_uri on documents (uri)