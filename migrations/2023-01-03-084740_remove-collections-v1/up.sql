-- Delete settings last modified logic
ALTER TABLE settings DROP COLUMN collections_last_modified_time;
DROP TRIGGER trigger_update_collections_last_modified on collections;
DROP FUNCTION update_last_modified;
 -- delete V1/V2 synchronization
 DROP TRIGGER trigger_update_collection_items ON collections;
DROP FUNCTION update_collection_item;

DROP TRIGGER trigger_sync_collection_items ON collections;
DROP FUNCTION synchronize_collection_items;

DROP TRIGGER trigger_sync_collections_old ON collection_items;
DROP FUNCTION synchronize_collections_old;

DROP TABLE collections;
