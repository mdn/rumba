-- This file should undo anything in `up.sql`
DROP TABLE multiple_collections_to_items;
DROP TABLE collection_items;
DROP TABLE multiple_collections;
DROP TRIGGER trigger_sync_collection_items ON mdn.public.collections;
DROP TRIGGER trigger_update_collection_items ON mdn.public.collections;
DROP FUNCTION synchronize_collection_items;