CREATE TABLE ai_explain_cache (
    id                  BIGSERIAL PRIMARY KEY,
    signature           bytea NOT NULL,
    highlighted_hash    bytea NOT NULL,
    language            VARCHAR(255),
    explanation         TEXT,
    created_at          TIMESTAMP NOT NULL DEFAULT now(),
    last_used           TIMESTAMP NOT NULL DEFAULT now(),
    view_count          BIGINT NOT NULL DEFAULT 1,
    version             BIGINT NOT NULL DEFAULT 1,
    thumbs_up           BIGINT NOT NULL DEFAULT 0,
    thumbs_down         BIGINT NOT NULL DEFAULT 0,
    UNIQUE(signature, highlighted_hash, version)
);