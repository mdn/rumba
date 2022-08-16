use crate::api::common::Sorting;
use crate::api::v2::multiple_collections::{
    CollectionItemCreationRequest, CollectionItemModificationRequest, CollectionItemQueryParams,
};
use crate::db::documents::create_or_update_document;
use crate::db::error::DbError;
use crate::db::model::{DocumentMetadata, IdQuery, UserQuery};
use crate::db::schema;
use crate::diesel::BoolExpressionMethods;
use crate::diesel::NullableExpressionMethods;
use crate::diesel::OptionalExtension;
use crate::diesel::PgTextExpressionMethods;
use crate::util::normalize_uri;
use chrono::{NaiveDateTime, Utc};
use diesel::dsl::{count, exists};
use diesel::expression_methods::ExpressionMethods;
use diesel::r2d2::ConnectionManager;
use diesel::{insert_into, select, PgConnection};
use diesel::{update, RunQueryDsl};
use diesel::{QueryDsl, QueryResult};
use r2d2::PooledConnection;

use super::model::{CollectionItemAndDocumentQuery, CollectionItemInsert};

pub fn get_collection_item_by_url(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &str,
) -> Result<CollectionItemAndDocumentQuery, DbError> {
    let collection: CollectionItemAndDocumentQuery = schema::collection_items::table
        .filter(schema::collection_items::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .filter(
            schema::documents::uri
                .eq(normalize_uri(url))
                .and(schema::collection_items::deleted_at.is_null()),
        )
        .select((
            schema::collection_items::id,
            schema::collection_items::created_at,
            schema::collection_items::updated_at,
            schema::collection_items::document_id,
            schema::collection_items::notes,
            schema::collection_items::custom_name,
            schema::collection_items::user_id,
            schema::documents::uri,
            schema::documents::metadata,
            schema::documents::title,
        ))
        .first::<CollectionItemAndDocumentQuery>(pool)?;

    Ok(collection)
}

pub fn get_collection_item_by_id(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    id: i64,
) -> Result<Option<CollectionItemAndDocumentQuery>, DbError> {
    let collection: Option<CollectionItemAndDocumentQuery> = schema::collection_items::table
        .filter(
            schema::collection_items::user_id
                .eq(user.id)
                .and(schema::collection_items::id.eq(id)),
        )
        .inner_join(schema::documents::table)
        .select((
            schema::collection_items::id,
            schema::collection_items::created_at,
            schema::collection_items::updated_at,
            schema::collection_items::document_id,
            schema::collection_items::notes,
            schema::collection_items::custom_name,
            schema::collection_items::user_id,
            schema::documents::uri,
            schema::documents::metadata,
            schema::documents::title,
        ))
        .first::<CollectionItemAndDocumentQuery>(pool)
        .optional()?;

    Ok(collection)
}

pub fn get_collection_items_paginated(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &CollectionItemQueryParams,
) -> Result<Vec<CollectionItemAndDocumentQuery>, DbError> {
    let mut collections_query = schema::collection_items::table
        .filter(
            schema::collection_items::user_id
                .eq(user.id)
                .and(schema::collection_items::deleted_at.is_null()),
        )
        .inner_join(schema::documents::table)
        .into_boxed();

    if let Some(query) = &query_params.q {
        collections_query = collections_query
            .filter(
                schema::collection_items::custom_name.is_not_null().and(
                    schema::collection_items::custom_name
                        .nullable()
                        .ilike(format!("%{}%", query)),
                ),
            )
            .or_filter(
                schema::collection_items::custom_name
                    .is_null()
                    .and(schema::documents::title.ilike(format!("%{}%", query))),
            )
            .or_filter(
                schema::collection_items::notes
                    .nullable()
                    .ilike(format!("%{}%", query)),
            );
    }

    collections_query = match query_params.sort {
        Some(Sorting::Title) => collections_query
            .order_by(schema::collection_items::custom_name.desc())
            .then_order_by(schema::documents::title.desc()),
        Some(Sorting::Created) => {
            collections_query.order_by(schema::collection_items::created_at.desc())
        }
        None => collections_query.order_by(schema::collection_items::created_at.desc()),
    };

    if let Some(limit) = query_params.limit {
        collections_query = collections_query.limit(limit.into())
    }

    if let Some(offset) = query_params.offset {
        collections_query = collections_query.offset(offset.into())
    }

    Ok(collections_query
        .select((
            schema::collection_items::id,
            schema::collection_items::created_at,
            schema::collection_items::updated_at,
            schema::collection_items::document_id,
            schema::collection_items::notes,
            schema::collection_items::custom_name,
            schema::collection_items::user_id,
            schema::documents::uri,
            schema::documents::metadata,
            schema::documents::title,
        ))
        .get_results::<CollectionItemAndDocumentQuery>(pool)?)
}

pub fn undelete_collection_item(
    user_id: i64,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &str,
) -> QueryResult<usize> {
    update(schema::collection_items::table)
        .filter(
            schema::collection_items::document_id.eq_any(
                schema::documents::table
                    .filter(schema::documents::uri.eq(normalize_uri(url)))
                    .select(schema::documents::id),
            ),
        )
        .filter(schema::collection_items::user_id.eq(user_id))
        .set(schema::collection_items::deleted_at.eq(None::<NaiveDateTime>))
        .execute(pool)
}

pub fn delete_collection_item_in_collection(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    id: i64,
) -> QueryResult<usize> {
    update(schema::collection_items::table)
        .filter(
            schema::collection_items::user_id
                .eq(user.id)
                .and(schema::collection_items::id.eq(id)),
        )
        .set(schema::collection_items::deleted_at.eq(chrono::offset::Utc::now().naive_utc()))
        .execute(pool)
}

pub fn create_collection_item(
    user_id: i64,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &str,
    document: DocumentMetadata,
    form: &CollectionItemCreationRequest,
    collection_id: i64,
) -> Result<i64, DbError> {
    let mut custom_name = None;

    if form.title != document.title {
        custom_name = Some(form.title.to_owned());
    }

    let url_normalized = normalize_uri(url);

    let document_id = create_or_update_document(pool, document, url_normalized)?;

    let collection_insert = CollectionItemInsert {
        document_id,
        notes: form.notes.clone(),
        custom_name,
        user_id,
        multiple_collection_id: collection_id,
    };

    let id_created = insert_into(schema::collection_items::table)
        .values(&collection_insert)
        .returning(schema::collection_items::id)
        .get_result::<i64>(pool);
    Ok(id_created?)
}

pub fn get_collection_item_count(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
) -> Result<i64, DbError> {
    let count = schema::collection_items::table
        .filter(
            schema::collection_items::user_id
                .eq(user_id)
                .and(schema::collection_items::deleted_at.is_null()),
        )
        .select(count(schema::collection_items::id))
        .first(pool)?;
    Ok(count)
}

pub fn collection_item_exists_for_user(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &str,
) -> Result<Option<IdQuery>, DbError> {
    let collection = schema::collection_items::table
        .filter(schema::collection_items::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .filter(
            schema::documents::uri
                .eq(normalize_uri(url))
                .and(schema::collection_items::deleted_at.is_null()),
        )
        .select((schema::collection_items::id,))
        .first::<IdQuery>(pool)
        .optional()?;

    Ok(collection)
}

pub fn multiple_collection_exists_for_user(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    collection_id: i64,
) -> Result<bool, DbError> {
    Ok(select(exists(
        schema::multiple_collections::table.filter(
            schema::multiple_collections::id
                .eq(collection_id)
                .and(schema::multiple_collections::user_id.eq(user.id)),
        ),
    ))
    .get_result(pool)?)
}

pub fn get_collection_items_for_user(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &str,
) -> Result<bool, DbError> {
    let collection = schema::collection_items::table
        .filter(schema::collection_items::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .filter(
            schema::documents::uri
                .eq(normalize_uri(url))
                .and(schema::collection_items::deleted_at.is_null()),
        )
        .select((schema::collection_items::id,))
        .first::<IdQuery>(pool)
        .optional()?;

    Ok(collection.is_some())
}

pub fn update_collection_item(
    collection_item_id: i64,
    user_id: i64,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    req: &CollectionItemModificationRequest,
) -> Result<usize, DbError> {
    Ok(update(
        schema::collection_items::table.filter(
            schema::collection_items::id
                .eq(collection_item_id)
                .and(schema::collection_items::user_id.eq(user_id)),
        ),
    )
    .set((
        schema::collection_items::notes.eq(&req.notes),
        schema::collection_items::custom_name.eq(&req.title),
        schema::collection_items::updated_at.eq(Utc::now().naive_utc()),
    ))
    .execute(pool)?)
}
