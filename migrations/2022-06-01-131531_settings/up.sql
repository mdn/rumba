CREATE TYPE locale AS ENUM (
	'de',
    'en_us',
    'es',
    'fr',
    'ja',
    'ko',
    'pl',
    'pt_br',
    'ru',
    'zh_cn',
    'zh_tw'
);

CREATE TABLE settings (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGSERIAL REFERENCES users (id),
	col_in_search   BOOLEAN NOT NULL DEFAULT FALSE,
	locale_override locale DEFAULT NULL,
    UNIQUE(user_id)
);