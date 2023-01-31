ALTER TABLE collections DROP CONSTRAINT collections_user_id_fkey,
    ADD FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE settings DROP CONSTRAINT settings_user_id_fkey,
    ADD FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE watched_items DROP CONSTRAINT watched_items_user_id_fkey,
    ADD FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE notifications DROP CONSTRAINT notifications_user_id_fkey,
    ADD FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;