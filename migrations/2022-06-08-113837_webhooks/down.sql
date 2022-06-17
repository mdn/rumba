-- This file should undo anything in `up.sql`
DROP TABLE raw_webhook_events_tokens;
DROP TABLE webhook_events;
DROP TYPE fxa_event_type;
DROP TYPE fxa_event_status_type;