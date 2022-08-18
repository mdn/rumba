-- This file should undo anything in `up.sql`
-- This creates a collection_item and adds it to the user's default collection any time they create a V1 collection.
CREATE OR REPLACE FUNCTION synchronize_collection_items()
    RETURNS TRIGGER AS
$$
BEGIN
    with USER_DEFAULT_COLLECTION as (select mcs.id      as collection_id,
                                            NEW.user_id as user_id
                                     from multiple_collections mcs
                                     where user_id = NEW.user_id
                                       and mcs.name = 'Default')
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
    where ci.user_id = NEW.user_id
      and ci.document_id = NEW.document_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
