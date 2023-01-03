-- This migration drops the data from the original collections table. 
-- It is no longer used in the API.
ALTER TABLE settings
    ADD COLUMN collections_last_modified_time TIMESTAMP DEFAULT NULL;    

CREATE TABLE collections
(
    id          BIGSERIAL PRIMARY KEY,
    created_at  TIMESTAMP NOT NULL DEFAULT now(),
    updated_at  TIMESTAMP NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMP,
    document_id BIGSERIAL references documents (id),
    notes       TEXT,
    custom_name TEXT,
    user_id     BIGSERIAL REFERENCES users (id),
    UNIQUE(document_id, user_id)
);

-- Trigger and function to update collection_items + default collections when old API is used.
CREATE OR REPLACE FUNCTION update_last_modified()
    RETURNS TRIGGER AS
$$
BEGIN
    IF NEW.deleted_at is not null THEN
        INSERT INTO settings (user_id, collections_last_modified_time)
        VALUES (NEW.user_id, NEW.deleted_at)
        ON CONFLICT (user_id) DO UPDATE set collections_last_modified_time = NEW.deleted_at
        where settings.user_id = NEW.user_id;
    ELSE
        INSERT INTO settings(user_id, collections_last_modified_time)
        VALUES (NEW.user_id, NEW.updated_at)
        ON CONFLICT (user_id)
            DO UPDATE SET collections_last_modified_time = NEW.updated_at
        WHERE settings.user_id = NEW.user_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_collections_last_modified
    AFTER INSERT OR UPDATE
    ON collections
    FOR EACH ROW
    WHEN (pg_trigger_depth() < 2) -- Either recurisve from collection_item update or direct
EXECUTE PROCEDURE update_last_modified();


-- Trigger and function to update collection_items + default collections when old API is used.
CREATE OR REPLACE FUNCTION update_collection_item()
    RETURNS TRIGGER AS
$$
BEGIN
    UPDATE collection_items ci
    set notes       = NEW.notes,
        custom_name = NEW.custom_name,
        deleted_at  = NEW.deleted_at,
        updated_at  = NEW.updated_at
    from(select id as collection_id from multiple_collections mc where user_id = NEW.user_id and mc.name = 'Default') as mcs  
    where ci.user_id = NEW.user_id
      and ci.document_id = NEW.document_id
      and multiple_collection_id = mcs.collection_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_collection_items
    AFTER UPDATE
    ON collections
    FOR EACH ROW
WHEN (pg_trigger_depth() = 0)    
EXECUTE PROCEDURE update_collection_item();

-- This creates a collection_item and adds it to the user's default collection any time they create a V1 collection.
CREATE OR REPLACE FUNCTION synchronize_collection_items()
    RETURNS TRIGGER AS
$$
BEGIN
    INSERT
    INTO collection_items (created_at,
                                      updated_at,
                                      deleted_at,
                                      document_id,
                                      notes,
                                      custom_name,
                                      user_id, multiple_collection_id)
    select NEW.created_at,
           NEW.updated_at,
           NEW.deleted_at,
           NEW.document_id,
           NEW.notes,
           NEW.custom_name,
           NEW.user_id,
           mcs.id
    from multiple_collections mcs
    where user_id = NEW.user_id
      and mcs.name = 'Default';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_sync_collection_items
    AFTER INSERT
    ON collections
    FOR EACH ROW
WHEN (pg_trigger_depth() = 0)
EXECUTE PROCEDURE synchronize_collection_items();

--This synchronizes everything added to the default collection back to the old v1 collections
CREATE OR REPLACE FUNCTION synchronize_collections_old()
    RETURNS TRIGGER AS
$$
BEGIN
    IF EXISTS (SELECT id as default_id
                        from multiple_collections
                        where user_id = NEW.user_id and name = 'Default' and id = NEW.multiple_collection_id)
        THEN INSERT into collections(created_at,
                                           updated_at,
                                           deleted_at,
                                           document_id,
                                           notes,
                                           custom_name,
                                           user_id)
        VALUES (NEW.created_at, NEW.updated_at, NEW.deleted_at, NEW.document_id, NEW.notes, new.custom_name, new.user_id)
        ON CONFLICT (document_id,user_id) DO UPDATE set custom_name = new.custom_name,
                                                        notes       = new.notes,
                                                        deleted_at  = new.deleted_at,
                                                        updated_at  = new.updated_at;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_sync_collections_old
    AFTER INSERT OR UPDATE
    ON collection_items
    FOR EACH ROW
WHEN (pg_trigger_depth() = 0)    
EXECUTE PROCEDURE synchronize_collections_old();
