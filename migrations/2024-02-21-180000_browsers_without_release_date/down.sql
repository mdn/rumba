DELETE FROM browser_releases WHERE release_date IS NULL;
ALTER TABLE browser_releases ALTER COLUMN release_date SET NOT NULL;
