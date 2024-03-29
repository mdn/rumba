-- Your SQL goes here
CREATE TYPE bcd_event_type AS ENUM 
(
    'added_stable', 
    'added_preview', 
    'added_subfeatures',
    'added_nonnull',
    'removed_stable',
    'unknown'
);

CREATE TABLE browsers 
(
    name                  TEXT PRIMARY KEY,
    display_name          TEXT NOT NULL,
    accepts_flags         BOOLEAN,
    accepts_webextensions BOOLEAN,
    pref_url              TEXT,
    preview_name          TEXT
);

CREATE TABLE browser_releases 
(
    id             BIGSERIAL PRIMARY KEY,
    browser        TEXT NOT NULL references browsers(name),
    engine         TEXT NOT NULL,
    engine_version TEXT NOT NULL,
    release_id     TEXT NOT NULL,
    release_date   DATE NOT NULL,
    release_notes  TEXT,
    status         TEXT,
    UNIQUE(browser, engine, release_date, release_id)
);

CREATE TABLE bcd_features 
(
    id BIGSERIAL   PRIMARY KEY,
    deprecated     BOOLEAN,    
    experimental   BOOLEAN,
    mdn_url        TEXT,
    path           TEXT NOT NULL UNIQUE,
    short_title    TEXT,
    source_file    TEXT NOT NULL,        
    spec_url       TEXT,    
    standard_track BOOLEAN
);

CREATE TABLE bcd_updates
(
    id                 BIGSERIAL PRIMARY KEY,    
    browser_release    BIGSERIAL REFERENCES browser_releases(id),
    created_at         TIMESTAMP NOT NULL DEFAULT now(),
    description        TEXT, 
    event_type         bcd_event_type NOT NULL,
    feature       BIGSERIAL REFERENCES bcd_features,
    UNIQUE(browser_release, feature)
);

CREATE TABLE bcd_updates_read_table
(
    id             BIGSERIAL PRIMARY KEY,    
    browser_name   TEXT           NOT NULL,
    browser        TEXT           NOT NULL,
    category       TEXT           NOT NULL,
    deprecated     BOOLEAN,
    description    TEXT,
    engine         TEXT           NOT NULL,
    engine_version TEXT           NOT NULL,
    event_type     bcd_event_type NOT NULL,
    experimental   BOOLEAN,
    mdn_url        TEXT,
    short_title    TEXT,
    path           TEXT           NOT NULL,
    release_date   DATE           NOT NULL,
    release_id     TEXT           NOT NULL,
    release_notes  TEXT,
    source_file    TEXT           NOT NULL,
    spec_url       TEXT,
    standard_track BOOLEAN,
    status         TEXT
);

CREATE INDEX release_date_idx ON bcd_updates_read_table ((release_date::DATE));
CREATE INDEX browser_name_idx ON bcd_updates_read_table ((browser::TEXT));
CREATE INDEX category_idx ON bcd_updates_read_table ((category::TEXT));

CREATE OR REPLACE FUNCTION update_bcd_update_view()
    RETURNS TRIGGER AS
$$
BEGIN
    INSERT INTO bcd_updates_read_table
         (SELECT
                NEXTVAL('bcd_updates_read_table_id_seq'),
                b.display_name,
                b.name,
                SPLIT_PART(f.path,'.',1),
                f.deprecated,
                NEW.description,
                br.engine,
                br.engine_version,
                NEW.event_type,
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
                br.status
         FROM browser_releases br
                  left join bcd_features f on f.id = NEW.feature
                  left join browsers b on br.browser = b.name
         where f.id = NEW.feature and NEW.browser_release = br.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trigger_update_bcd_update_view
    AFTER INSERT
    ON bcd_updates
    FOR EACH ROW
EXECUTE PROCEDURE update_bcd_update_view();