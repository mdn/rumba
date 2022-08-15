-- This file should undo anything in `up.sql`
DROP TRIGGER trigger_sync_collections_old ON mdn.public.collection_items;
DROP TABLE collection_items;
DROP TABLE multiple_collections;
DROP TRIGGER trigger_sync_collection_items ON mdn.public.collections;
DROP TRIGGER trigger_update_collection_items ON mdn.public.collections;
DROP FUNCTION synchronize_collection_items;
DROP FUNCTION synchronize_collections_old;
DROP INDEX multiple_collection_unique_name_user_not_deleted;
DROP INDEX collection_items_unique_to_user_multiple_collection_not_deleted;