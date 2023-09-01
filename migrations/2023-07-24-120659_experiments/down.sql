DROP TABLE experiments;

ALTER TABLE users
    DROP COLUMN is_mdn_team,
    DROP COLUMN is_fox_food; 