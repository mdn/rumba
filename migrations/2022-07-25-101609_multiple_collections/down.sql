-- This file should undo anything in `up.sql`
DROP TRIGGER trigger_sync_collections_old ON collection_items;
DROP TABLE collection_items;
DROP TABLE multiple_collections;
DROP TRIGGER trigger_sync_collection_items ON collections;
DROP TRIGGER trigger_update_collection_items ON collections;
DROP FUNCTION synchronize_collection_items;
DROP FUNCTION synchronize_collections_old;