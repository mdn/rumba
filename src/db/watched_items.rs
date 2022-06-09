use diesel::{delete, insert_into};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;

use crate::api::watched_items::WatchedItemQueryParams;

use super::documents::create_or_update_document;
use super::model::{DocumentMetadata, WatchedItemInsert};
use super::{error::DbError, model::WatchedItemsQuery, schema};

use diesel::prelude::*;
use diesel::QueryDsl;
use diesel::RunQueryDsl;

use crate::diesel::PgTextExpressionMethods;
use diesel::expression_methods::ExpressionMethods;

pub async fn get_watched_items(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    query: &WatchedItemQueryParams,
) -> Result<Vec<WatchedItemsQuery>, DbError> {
    let mut watched_items_query = schema::watched_items::table
        .filter(schema::watched_items::user_id.eq(user_id))
        .inner_join(
            schema::documents::table
                .on(schema::documents::id.eq(schema::watched_items::document_id)),
        )
        .into_boxed();

    if let Some(filter) = &query.q {
        watched_items_query =
            watched_items_query.filter(schema::documents::title.ilike(format!("%{}%", filter)))
    }

    if let Some(limit) = query.limit {
        watched_items_query = watched_items_query.limit(limit.into())
    } else {
        watched_items_query = watched_items_query.limit(10)
    }

    if let Some(offset) = query.offset {
        watched_items_query = watched_items_query.offset(offset.into())
    }

    Ok(watched_items_query
        .select((
            schema::watched_items::user_id,
            schema::watched_items::document_id,
            schema::documents::created_at,
            schema::documents::uri,
            schema::documents::title,
            schema::documents::paths,
        ))
        .order_by(schema::watched_items::created_at.desc())
        .get_results::<WatchedItemsQuery>(pool)?)
}

pub async fn get_watched_item(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    url: &String,
) -> Result<Option<WatchedItemsQuery>, DbError> {
    let item = schema::watched_items::table
        .filter(schema::watched_items::user_id.eq(user_id))
        .inner_join(
            schema::documents::table
                .on(schema::watched_items::document_id.eq(schema::documents::id)),
        )
        .filter(schema::documents::uri.eq(url))
        .select((
            schema::watched_items::document_id,
            schema::watched_items::user_id,
            schema::documents::created_at,
            schema::documents::uri,
            schema::documents::title,
            schema::documents::paths,
        ))
        .first::<WatchedItemsQuery>(pool)
        .optional()?;
    Ok(item)
}

pub async fn create_watched_item(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    metadata: DocumentMetadata,
    url: String,
) -> QueryResult<usize> {
    let document_id = create_or_update_document(pool, metadata, url).await?;
    let insert = WatchedItemInsert {
        document_id,
        user_id,
    };
    let inserted_rows = insert_into(schema::watched_items::table)
        .values(&insert)
        .on_conflict_do_nothing()
        .execute(pool)?;
    Ok(inserted_rows)
}

pub async fn delete_watched_items(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    urls: Vec<String>,
) -> QueryResult<usize> {
    delete(schema::watched_items::table)
        .filter(schema::watched_items::user_id.eq(user_id))
        .filter(
            schema::watched_items::document_id.eq_any(
                schema::documents::table
                    .filter(schema::documents::uri.eq_any(urls))
                    .select(schema::documents::id),
            ),
        )
        .execute(pool)
}

pub async fn delete_watched_item(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    document_id: i64,
) -> QueryResult<usize> {
    delete(
        schema::watched_items::table
            .filter(schema::watched_items::user_id.eq(user_id))
            .filter(schema::watched_items::document_id.eq(document_id)),
    )
    .execute(pool)
}
