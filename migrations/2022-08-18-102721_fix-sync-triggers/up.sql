-- Your SQL goes here
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