use crate::api::collections::CollectionsQueryParams;
use crate::db::error::DbError;
use crate::db::model::{CollectionAndDocumentQuery, UserQuery};
use crate::db::schema;
use crate::diesel::BoolExpressionMethods;
use crate::diesel::NullableExpressionMethods;
use crate::diesel::PgTextExpressionMethods;
use diesel::expression_methods::ExpressionMethods;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
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
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &CollectionsQueryParams,
) -> Result<Vec<CollectionAndDocumentQuery>, DbError> {
    let mut collections_query = schema::collections::table
        .filter(schema::collections::user_id.eq(user.id))
        .inner_join(schema::documents::table)
        .into_boxed();

    if query_params.q.is_some() {
        let query = query_params.q.as_ref().unwrap();
        collections_query = collections_query
            .filter(
                schema::collections::custom_name.is_not_null().and(
                    schema::collections::custom_name
                        .nullable()
                        .ilike(query.to_owned() + "%"),
                ),
            )
            .or_filter(
                schema::collections::custom_name
                    .is_null()
                    .and(schema::documents::title.ilike(query.to_owned() + "%")),
            );
    }

    if query_params.limit.is_some() {
        collections_query = collections_query.limit(query_params.limit.unwrap().into())
    }

    if query_params.offset.is_some() {
        collections_query = collections_query.offset(query_params.offset.unwrap().into())
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
