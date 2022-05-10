use crate::api::collections::CollectionsQueryParams;
use crate::db::model::{CollectionAndDocumentQuery, CollectionQuery, DocumentQuery, UserQuery};
use crate::db::schema;
use crate::db::Pool;
use actix_web::{web, HttpRequest};

use crate::db::error::DbError;
use diesel::expression_methods::ExpressionMethods;
use diesel::r2d2::ConnectionManager;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use r2d2::PooledConnection;

pub async fn get_collection(
    user: UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    url: &String,
) -> Result<CollectionAndDocumentQuery, DbError> {
    let collection: CollectionAndDocumentQuery = schema::collections::table
        .filter(schema::collections::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .filter(schema::documents::uri.eq(url))
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
    pool: web::Data<Pool>,
    query_params: &CollectionsQueryParams,
) -> Result<Vec<CollectionQuery>, DbError> {
    let collections: Vec<CollectionAndDocumentQuery> = schema::collections::table
        .filter(schema::collections::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .filter(schema::documents::uri.eq(query_params.url))

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
        ))
        .get_results::<CollectionAndDocumentQuery>(pool)?;
    Err(DbError::DieselResult(diesel::result::Error::NotFound))
}
