-- Your SQL goes here
CREATE TYPE engine_type AS ENUM (
    'gecko',
    'webkit',
    'blink',
    'presto',
    'edgehtml',
    'trident',
    'unknown'
);
ALTER TABLE bcd_updates
ADD COLUMN engines engine_type [] NOT NULL DEFAULT '{}';

CREATE MATERIALIZED VIEW bcd_updates_view AS
SELECT 
    b.display_name as browser_name,
    b.name as browser,
    SPLIT_PART(f.path, '.', 1) as category,
    f.deprecated,
    up.description,
    br.engine,
    br.engine_version,
    up.event_type,
    f.experimental,
    f.mdn_url,
    f.short_title,
    f.path,
    br.release_date,
    br.release_id,
    br.release_notes,
    f.source_file,
    f.spec_url,
    f.standard_track,
    br.status,
    up.engines
FROM bcd_updates up
    left join browser_releases br on up.browser_release = br.id
    left join bcd_features f on f.id = up.feature
    left join browsers b on br.browser = b.name;

CREATE UNIQUE INDEX buv_unique_idx ON bcd_updates_view ((browser::TEXT), (event_type::bcd_event_type), (release_id::TEXT), (path::TEXT));
CREATE INDEX buv_release_date_idx ON bcd_updates_view ((release_date::DATE));
CREATE INDEX buv_browser_name_idx ON bcd_updates_view ((browser::TEXT));
CREATE INDEX buv_category_idx ON bcd_updates_view ((category::TEXT));
CREATE INDEX buv_bcd_updates_lower_case_url_idx ON bcd_updates_view ((lower(mdn_url)));


DROP INDEX release_date_idx,browser_name_idx, category_idx;
DROP TRIGGER trigger_update_bcd_update_view ON bcd_updates;
DROP TABLE bcd_updates_read_table;
DROP FUNCTION update_bcd_update_view;
