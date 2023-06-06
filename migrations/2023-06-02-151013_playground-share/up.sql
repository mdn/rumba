CREATE TABLE playground (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NULL REFERENCES users (id) ON DELETE SET NULL,
    gist            TEXT NOT NULL UNIQUE,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    flagged         BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_user_id BIGINT DEFAULT NULL
);

CREATE INDEX playground_gist ON playground (gist);

CREATE FUNCTION set_deleted_user_id() RETURNS trigger AS $$
    BEGIN
        IF OLD.user_id IS NOT NULL AND NEW.user_id IS NULL THEN
            NEW.deleted_user_id := OLD.user_id;
        END IF;
        RETURN NEW;
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_deleted_user_id
BEFORE UPDATE ON playground 
FOR EACH ROW
EXECUTE PROCEDURE set_deleted_user_id();
