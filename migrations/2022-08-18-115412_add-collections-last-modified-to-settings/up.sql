ALTER TABLE settings
    ADD COLUMN collections_last_modified_time TIMESTAMP DEFAULT NULL;

WITH COLLECTION_LAST_MODIFIED_USER AS (
    SELECT user_id, max(collections.updated_at) as modified_time
    from collections
    group by user_id
)
INSERT
INTO settings(user_id, collections_last_modified_time)
    (SELECT COLLECTION_LAST_MODIFIED_USER.user_id, COLLECTION_LAST_MODIFIED_USER.modified_time
     from COLLECTION_LAST_MODIFIED_USER)
ON CONFLICT(user_id) DO UPDATE SET collections_last_modified_time = (SELECT COLLECTION_LAST_MODIFIED_USER.modified_time
                                                                     from COLLECTION_LAST_MODIFIED_USER
                                                                     where user_id = EXCLUDED.user_id);

-- Trigger and function to update collection_items + default collections when old API is used.
CREATE OR REPLACE FUNCTION update_last_modified()
    RETURNS TRIGGER AS
$$
BEGIN
    IF NEW.deleted_at is not null THEN
        INSERT INTO settings (user_id, collections_last_modified_time)
        VALUES (NEW.user_id, NEW.deleted_at)
        ON CONFLICT (user_id) DO UPDATE set collections_last_modified_time = NEW.deleted_at
        where settings.user_id = NEW.user_id;
    ELSE
        INSERT INTO settings(user_id, collections_last_modified_time)
        VALUES (NEW.user_id, NEW.updated_at)
        ON CONFLICT (user_id)
            DO UPDATE SET collections_last_modified_time = NEW.updated_at
        WHERE settings.user_id = NEW.user_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_collections_last_modified
    AFTER INSERT OR UPDATE
    ON collections
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 2) -- Either recurisve from collection_item update or direct
EXECUTE PROCEDURE update_last_modified();
