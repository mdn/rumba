-- This file should undo anything in `up.sql`
DROP INDEX release_date_idx,browser_name_idx;
DROP TRIGGER trigger_update_bcd_update_view ON bcd_updates;
DROP TABLE bcd_updates;
DROP TABLE browser_releases;
DROP TABLE features;
DROP TABLE bcd_updates_view;
DROP TABLE browsers;
DROP FUNCTION update_bcd_update_view;
DROP TYPE  bcd_event_type;
