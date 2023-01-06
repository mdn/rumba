-- Optimize for Collections queries.
CREATE INDEX bcd_updates_lower_case_url_idx ON bcd_updates_read_table ((lower(mdn_url)));

-- Get all unique user id's in wathcing and create them a 'Watched items collection'
WITH users_watching AS (
    SELECT distinct user_id
    from watched_items
)
INSERT
INTO multiple_collections(created_at, updated_at, deleted_at, user_id, notes, name)
select now(),
       now(),
       null,
       users_watching.user_id,
       'Articles you are watching',
       'Watched items'
FROM users_watching
ON CONFLICT DO NOTHING;

-- Add all watched items to that collection. Backup migrated values.
WITH watching AS (
    SELECT *
    from watched_items
)
INSERT INTO collection_items (created_at, updated_at, deleted_at, document_id, user_id, notes, custom_name,
                                multiple_collection_id)
             SELECT now(),
                    now(),
                    null,
                    watching.document_id,
                    watching.user_id,
                    null,
                    null,
                    mcs.id as mcs_id            
             FROM watching watching
                      LEFT JOIN multiple_collections mcs
                                ON mcs.name = 'Watched items' and mcs.user_id = watching.user_id
ON CONFLICT DO NOTHING;
