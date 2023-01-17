BEGIN;
ALTER TABLE activity_pings DROP CONSTRAINT activity_pings_user_id_fkey;
ALTER TABLE activity_pings ADD FOREIGN KEY (user_id)
REFERENCES users(id) ON DELETE NO ACTION;
COMMIT;
