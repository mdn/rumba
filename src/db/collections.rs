use crate::api::collections::{CollectionCreationForm, CollectionsQueryParams, Sorting};
use crate::db::documents::create_or_update_document;
use crate::db::error::DbError;
use crate::db::model::{CollectionAndDocumentQuery, CollectionInsert, DocumentMetadata, UserQuery};
use crate::db::schema;
use crate::diesel::BoolExpressionMethods;
use crate::diesel::NullableExpressionMethods;
use crate::diesel::PgTextExpressionMethods;
use diesel::expression_methods::ExpressionMethods;
use diesel::r2d2::ConnectionManager;
use diesel::{update, RunQueryDsl};
use diesel::{insert_into, PgConnection};
use diesel::{QueryDsl, QueryResult};
use r2d2::PooledConnection;

pub async fn get_collection(
    user: UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &String,
) -> Result<CollectionAndDocumentQuery, DbError> {
    let collection: CollectionAndDocumentQuery = schema::collections::table
        .filter(schema::collections::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .filter(
            schema::documents::uri
                .eq(normalize_uri(url.to_string()))
                .and(schema::collections::deleted_at.is_null()),
        )
        .select((
            schema::collections::id,
            schema::collections::created_at,
            schema::collections::updated_at,
            schema::collections::document_id,
            schema::collections::notes,
            schema::collections::custom_name,
            schema::collections::user_id,
            schema::documents::uri,
            schema::documents::metadata,
            schema::documents::title,
        ))
        .first::<CollectionAndDocumentQuery>(pool)?;

    Ok(collection)
}

pub async fn get_collections_paginated(
    user: UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &CollectionsQueryParams,
) -> Result<Vec<CollectionAndDocumentQuery>, DbError> {
    let mut collections_query = schema::collections::table
        .filter(
            schema::collections::user_id
                .eq(user.id)
                .and(schema::collections::deleted_at.is_null()),
        )
        .inner_join(schema::documents::table)
        .into_boxed();

    if let Some(query) = &query_params.q {
        collections_query = collections_query
            .filter(
                schema::collections::custom_name.is_not_null().and(
                    schema::collections::custom_name
                        .nullable()
                        .ilike(format!("%{}%", query)),
                ),
            )
            .or_filter(
                schema::collections::custom_name
                    .is_null()
                    .and(schema::documents::title.ilike(format!("%{}%", query))),
            )
            .or_filter(
                schema::collections::notes
                    .nullable()
                    .ilike(format!("%{}%", query)),
            );
    }

    collections_query = match query_params.sort {
        Some(Sorting::Title) => collections_query
            .order_by(schema::collections::custom_name.desc())
            .then_order_by(schema::documents::title.desc()),
        Some(Sorting::Created) => {
            collections_query.order_by(schema::collections::created_at.desc())
        }
        None => collections_query.order_by(schema::collections::created_at.desc()),
    };

    if let Some(limit) = query_params.limit {
        collections_query = collections_query.limit(limit.into())
    }

    if let Some(offset) = query_params.offset {
        collections_query = collections_query.offset(offset.into())
    }

    Ok(collections_query
        .select((
            schema::collections::id,
            schema::collections::created_at,
            schema::collections::updated_at,
            schema::collections::document_id,
            schema::collections::notes,
            schema::collections::custom_name,
            schema::collections::user_id,
            schema::documents::uri,
            schema::documents::metadata,
            schema::documents::title,
        ))
        .get_results::<CollectionAndDocumentQuery>(pool)?)
}

pub async fn delete_collection_item(
    user: UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: String,
) -> QueryResult<usize> {
    update(schema::collections::table)
        .filter(
            schema::collections::document_id.eq_any(
                schema::documents::table
                    .filter(schema::documents::uri.eq(normalize_uri(url)))
                    .select(schema::documents::id),
            ),
        )
        .filter(schema::collections::user_id.eq(user.id))
        .set(schema::collections::deleted_at.eq(chrono::offset::Utc::now().naive_utc()))
        .execute(pool)
}

fn normalize_uri(input: String) -> String {
    input.to_lowercase().trim().to_string()
}

pub async fn create_collection(
    user: UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: String,
    document: DocumentMetadata,
    form: CollectionCreationForm,
) -> QueryResult<usize> {
    let mut custom_name = None;

    if form.name != document.title {
        custom_name = Some(form.name);
    }

    let url_normalized = normalize_uri(url);

    let document_id = create_or_update_document(pool, document, url_normalized)?;

    let collection_insert = CollectionInsert {
        document_id,
        notes: form.notes,
        custom_name,
        user_id: user.id,
    };

    insert_into(schema::collections::table)
        .values(&collection_insert)
        .on_conflict((
            schema::collections::user_id,
            schema::collections::document_id,
        ))
        .do_update()
        .set(&collection_insert)
        .execute(pool)
}
