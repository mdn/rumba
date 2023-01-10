ALTER TABLE settings DROP COLUMN multiple_collections;
ALTER TABLE settings DROP COLUMN col_in_search;
ALTER TABLE settings ADD COLUMN mdnplus_newsletter BOOLEAN NOT NULL DEFAULT FALSE;