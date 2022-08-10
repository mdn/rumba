CREATE TABLE multiple_collections
(
    id         BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    deleted_at TIMESTAMP,
    user_id    BIGSERIAL references users (id) ON DELETE CASCADE,
    notes      TEXT,
    name       TEXT      NOT NULL
);

--This is the same as 'Collections' but without the uniqueness constrain on user_id , document_id
CREATE TABLE collection_items
(
    id          BIGSERIAL PRIMARY KEY,
    created_at  TIMESTAMP NOT NULL DEFAULT now(),
    updated_at  TIMESTAMP NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMP,
    document_id BIGSERIAL references documents (id),
    notes       TEXT,
    custom_name TEXT,
    user_id     BIGSERIAL REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE multiple_collections_to_items
(
    multiple_collection_id BIGSERIAL references multiple_collections (id) ON DELETE CASCADE,
    collection_item_id    BIGSERIAL references collection_items (id),
    PRIMARY KEY (multiple_collection_id, collection_item_id)
);

-- Create default collection for every user
with users as (
    select id as user_id
    from mdn.public.users
)
insert
into mdn.public.multiple_collections(created_at, updated_at, deleted_at, user_id, notes, name)
select now(),
       now(),
       null,
       users.user_id,
       '',
       'Default'
from users;

-- Migrate collections to collection_items
with collections_old as (
    select id,
           created_at,
           updated_at,
           deleted_at,
           document_id,
           notes,
           custom_name,
           user_id
    from mdn.public.collections
)
insert
into mdn.public.collection_items(id, created_at, updated_at, deleted_at, document_id, user_id, notes, custom_name)
select id, created_at, updated_at, deleted_at, document_id, user_id, notes, custom_name
from collections_old;

-- Increment collection_items sequence.
SELECT setval('collection_items_id_seq', (SELECT max(id) from collection_items));

--Migrate all to the default collection
with users_collection_items as (
    select users.id as user_id,
           ci.id    as collection_item_id,
           mcs.id   as multiple_collections_id
    from mdn.public.users
             inner join mdn.public.collection_items ci on ci.user_id = users.id
             left join mdn.public.multiple_collections mcs on mcs.user_id = users.id
)
insert
into mdn.public.multiple_collections_to_items(multiple_collection_id, collection_item_id)
select users_collection_items.multiple_collections_id, users_collection_items.collection_item_id
from users_collection_items;

-- This creates a collection_item and adds it to the user's default collection any time they create a V1 collection.
CREATE OR REPLACE FUNCTION synchronize_collection_items()
    RETURNS TRIGGER AS
$$
BEGIN
    with NEW_COLLECTION_ITEM as (INSERT INTO mdn.public.collection_items (created_at,
                                                                          updated_at,
                                                                          deleted_at,
                                                                          document_id,
                                                                          notes,
                                                                          custom_name,
                                                                          user_id)
        VALUES (NEW.created_at, NEW.updated_at, NEW.deleted_at, NEW.document_id, NEW.notes, NEW.custom_name,
                NEW.user_id)
        RETURNING id as NEW_ITEM_ID, NEW.USER_ID),
         USER_DEFAULT_COLLECTION as (select mcs.id      as collection_id,
                                            NEW.user_id as user_id
                                     from multiple_collections mcs
                                     where user_id = NEW.user_id
                                       and mcs.name = 'Default')
    INSERT
    INTO mdn.public.multiple_collections_to_items (multiple_collection_id, collection_item_id)
    select USER_DEFAULT_COLLECTION.collection_id, nci.NEW_ITEM_ID
    from USER_DEFAULT_COLLECTION
             full outer join NEW_COLLECTION_ITEM nci on nci.user_id = USER_DEFAULT_COLLECTION.user_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger and function to update collection_items + default collections when old API is used.
CREATE OR REPLACE FUNCTION update_collection_item()
    RETURNS TRIGGER AS
$$
BEGIN
    UPDATE mdn.public.collection_items ci set notes = NEW.notes, custom_name = NEW.custom_name, deleted_at = NEW.deleted_at, updated_at = NEW.updated_at where ci.user_id = NEW.user_id and ci.document_id = NEW.document_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_sync_collection_items
    AFTER INSERT
    ON mdn.public.collections
    FOR EACH ROW
EXECUTE PROCEDURE synchronize_collection_items();


CREATE TRIGGER trigger_update_collection_items
    AFTER UPDATE
    ON mdn.public.collections
    FOR EACH ROW
EXECUTE PROCEDURE update_collection_item();
