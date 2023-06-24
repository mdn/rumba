CREATE TABLE ai_explain_cache (
    id                  BIGSERIAL PRIMARY KEY,
    signature           bytea NOT NULL,
    highlighted_hash    bytea NOT NULL,
    explanation         TEXT,
    created_at          TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE(signature, highlighted_hash)
);