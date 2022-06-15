-- Your SQL goes here

CREATE TYPE fxa_event_type AS ENUM ('delete_user', 'password_change', 'profile_change','subscription_state_change', 'unknown');
CREATE TYPE fxa_event_status_type AS ENUM ('processed', 'pending', 'ignored', 'failed');

CREATE TABLE webhook_events (
    id          BIGSERIAL PRIMARY KEY,
    fxa_uid     VARCHAR(255) NOT NULL,
    change_time TIMESTAMP,
    issue_time  TIMESTAMP NOT NULL,
	typ         fxa_event_type NOT NULL DEFAULT 'unknown',
    status      fxa_event_status_type NOT NULL,
    payload     JSONB NOT NULL
);