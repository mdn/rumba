DROP TABLE experiments;

ALTER TABLE users
    DROP COLUMN is_mdn_team,
    DROP COLUMN is_fox_food; 

DROP TABLE ai_help_logs;