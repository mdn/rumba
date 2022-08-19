ALTER TABLE settings
    DROP COLUMN collections_last_modified_time;

DROP TRIGGER trigger_update_collections_last_modified on collections;
DROP FUNCTION update_last_modified;