BEGIN;
ALTER TABLE collections DROP CONSTRAINT collections_user_id_fkey;
ALTER TABLE collections ADD FOREIGN KEY (user_id)
REFERENCES users(id) ON DELETE NO ACTION;
COMMIT;

BEGIN;
ALTER TABLE settings DROP CONSTRAINT settings_user_id_fkey;
ALTER TABLE settings ADD FOREIGN KEY (user_id)
REFERENCES users(id) ON DELETE NO ACTION;
COMMIT;

BEGIN;
ALTER TABLE watched_items DROP CONSTRAINT watched_items_user_id_fkey;
ALTER TABLE watched_items ADD FOREIGN KEY (user_id)
REFERENCES users(id) ON DELETE NO ACTION;
COMMIT;

BEGIN;
ALTER TABLE notifications DROP CONSTRAINT notifications_user_id_fkey;
ALTER TABLE notifications ADD FOREIGN KEY (user_id)
REFERENCES users(id) ON DELETE NO ACTION;
COMMIT;