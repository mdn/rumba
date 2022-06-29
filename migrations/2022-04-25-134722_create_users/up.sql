-- Your SQL goes here
CREATE TYPE subscription_type AS ENUM ('core', 'mdn_plus_5m', 'mdn_plus_5y', 'mdn_plus_10m','mdn_plus_10y', 'unknown');

CREATE TABLE users
(
    id                BIGSERIAL PRIMARY KEY,
    created_at        TIMESTAMP    NOT NULL DEFAULT now(),
    updated_at        TIMESTAMP    NOT NULL DEFAULT now(),
    email             TEXT         NOT NULL,
    fxa_uid           VARCHAR(255) NOT NULL UNIQUE,
    fxa_refresh_token VARCHAR(255) NOT NULL,
    avatar_url        TEXT,
    subscription_type subscription_type,
    enforce_plus      subscription_type,
    is_admin          BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX fxa_id
    on users (fxa_uid)