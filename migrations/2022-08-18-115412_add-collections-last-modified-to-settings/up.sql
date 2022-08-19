ALTER TABLE settings
    ADD COLUMN collections_last_modified_time TIMESTAMP DEFAULT NULL;

WITH COLLECTION_LAST_MODIFIED_USER AS (
    SELECT users.id as user_id, max(collections.updated_at) as modified_time
    from users
             left join collections on users.id = collections.user_id
             left join collection_items on users.id = collection_items.user_id
    where collections.updated_at is not null
    group by users.id
)
INSERT
INTO settings(user_id, collections_last_modified_time)
    (SELECT COLLECTION_LAST_MODIFIED_USER.user_id, COLLECTION_LAST_MODIFIED_USER.modified_time
     from COLLECTION_LAST_MODIFIED_USER)
ON CONFLICT(user_id) DO UPDATE SET collections_last_modified_time = (SELECT COLLECTION_LAST_MODIFIED_USER.modified_time
                                                                     from COLLECTION_LAST_MODIFIED_USER);

-- Trigger and function to update collection_items + default collections when old API is used.
CREATE OR REPLACE FUNCTION update_last_modified()
    RETURNS TRIGGER AS
$$
BEGIN
    IF NEW.deleted_at is not null THEN
        UPDATE settings
        set collections_last_modified_time = NEW.deleted_at
        where settings.user_id = NEW.user_id;
    ELSE
        UPDATE settings
        set collections_last_modified_time = NEW.updated_at
        where settings.user_id = NEW.user_id;
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
