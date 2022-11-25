-- Your SQL goes here
CREATE TYPE bcd_event_type AS ENUM (
    'added_stable', 
    'added_preview', 
    'added_subfeatures',
    'added_nonnull',
    'removed_stable',
    'unknown'
    );

CREATE TYPE browser_type AS ENUM (
    'chrome',
    'chrome_android',
    'deno',
    'edge',
    'firefox',
    'firefox_android',    
    'internet_explorer',
    'node_js',
    'opera',
    'opera_android',
    'safari',
    'safari_ios',
    'samsung_internet_android',
    'webview_android',    
    'unknown'
    );

CREATE TABLE bcd_update_history
(
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT now(),
    version_identifier  TEXT,
    status              TEXT  
);

CREATE TABLE bcd_updates
(
    id                 BIGSERIAL PRIMARY KEY,
    bcd_path           TEXT,
    browser            browser_type NOT NULL,
    created_at         TIMESTAMP NOT NULL DEFAULT now(),
    bcd_update_version BIGSERIAL references bcd_update_history(id),
    description        TEXT,
    mdn_url            TEXT,
    document_id        BIGSERIAL references documents(id),
    spec_url           TEXT,
    event_type         bcd_event_type NOT NULL
);
