CREATE TABLE activity_pings
(
    id       BIGSERIAL PRIMARY KEY,
    user_id  BIGSERIAL REFERENCES users (id),
    ping_at  TIMESTAMP NOT NULL DEFAULT date_trunc('day', now()),
    activity JSONB NOT NULL,
    UNIQUE(user_id, ping_at)
);
