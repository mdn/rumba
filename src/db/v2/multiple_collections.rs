use crate::api::common::Sorting;
use crate::api::v2::multiple_collections::CollectionItemQueryParams;
use crate::api::v2::multiple_collections::MultipleCollectionCreationRequest;
use crate::db::error::DbError;
use crate::db::model::UserQuery;
use crate::db::schema;
use crate::diesel::BoolExpressionMethods;
use crate::diesel::JoinOnDsl;
use crate::diesel::NullableExpressionMethods;
use crate::diesel::OptionalExtension;
use crate::diesel::PgTextExpressionMethods;

use diesel::dsl::count;
use diesel::update;
use diesel::QueryDsl;
use diesel::QueryResult;
use diesel::RunQueryDsl;
use diesel::{dsl::exists, expression_methods::ExpressionMethods};
use diesel::{insert_into, select};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;

use super::model::CollectionItemAndDocumentQuery;
use super::model::MultipleCollectionInsert;
use super::model::MultipleCollectionsQuery;
use super::model::MultipleCollectionsQueryNoCount;

pub fn get_multiple_collections_for_user(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<Vec<MultipleCollectionsQuery>, DbError> {
    let collections: Vec<MultipleCollectionsQuery> = schema::multiple_collections::table
        .filter(
            schema::multiple_collections::user_id
                .eq(user.id)
                .and(schema::multiple_collections::deleted_at.is_null())
                .and(schema::collection_items::deleted_at.is_null()),
        )
        .left_join(schema::collection_items::table)
        .group_by(schema::multiple_collections::id)
        .select((
            schema::multiple_collections::id,
            schema::multiple_collections::created_at,
            schema::multiple_collections::updated_at,
            schema::multiple_collections::deleted_at,
            schema::multiple_collections::user_id,
            schema::multiple_collections::notes,
            schema::multiple_collections::name,
            count(schema::collection_items::id).nullable(),
        ))
        .get_results::<MultipleCollectionsQuery>(pool)?;

    Ok(collections)
}

pub fn create_multiple_collection_for_user(
    pool: &mut PgConnection,
    user_id: i64,
    data: &MultipleCollectionCreationRequest,
) -> Result<MultipleCollectionsQuery, DbError> {
    let insert = MultipleCollectionInsert {
        deleted_at: None,
        name: data.name.to_owned(),
        notes: data.description.to_owned(),
        user_id,
    };
    //MultipleCollectionsQueryNoCount prevents type errors with returning all columns of the created object (as count of collection items is missing)
    let res = insert_into(schema::multiple_collections::table)
        .values(insert)
        .returning(schema::multiple_collections::all_columns)
        .get_result::<MultipleCollectionsQueryNoCount>(pool)?;
    Ok(res.into())
}

pub fn get_multiple_collection_by_id_for_user(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    id: &i64,
) -> Result<Option<MultipleCollectionsQuery>, DbError> {
    let collection = schema::multiple_collections::table
        .filter(
            schema::multiple_collections::user_id
                .eq(user.id)
                .and(schema::multiple_collections::deleted_at.is_null())
                .and(schema::multiple_collections::id.eq(id)),
        )
        .left_join(
            schema::collection_items::table.on(schema::collection_items::multiple_collection_id
                .eq(id)
                .and(schema::collection_items::deleted_at.is_null())),
        )
        .group_by(schema::multiple_collections::id)
        .select((
            schema::multiple_collections::id,
            schema::multiple_collections::created_at,
            schema::multiple_collections::updated_at,
            schema::multiple_collections::deleted_at,
            schema::multiple_collections::user_id,
            schema::multiple_collections::notes,
            schema::multiple_collections::name,
            count(schema::collection_items::id).nullable(),
        ))
        .first::<MultipleCollectionsQuery>(pool)
        .optional()?;
    Ok(collection)
}

pub fn multiple_collection_exists(
    user: &UserQuery,
    multiple_collection_id: &i64,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<bool, DbError> {
    let exists = select(exists(
        schema::multiple_collections::table.filter(
            schema::multiple_collections::id
                .eq(multiple_collection_id)
                .and(schema::multiple_collections::user_id.eq(user.id)),
        ),
    ))
    .get_result(pool);
    Ok(exists?)
}

pub fn create_default_multiple_collection_for_user(
    pool: &mut PgConnection,
    user_id: i64,
) -> Result<usize, diesel::result::Error> {
    let insert = MultipleCollectionInsert {
        deleted_at: None,
        name: "Default".to_string(),
        notes: None,
        user_id,
    };
    let res = insert_into(schema::multiple_collections::table)
        .values(insert)
        .execute(pool)?;
    Ok(res)
}

pub fn get_collection_items_for_user_multiple_collection(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    collection_id: &i64,
    query_params: &CollectionItemQueryParams,
) -> Result<Vec<CollectionItemAndDocumentQuery>, DbError> {
    let mut collections_query = schema::collection_items::table
        .inner_join(schema::documents::table)
        .filter(
            schema::collection_items::user_id
                .eq(user.id)
                .and(schema::collection_items::deleted_at.is_null())
                .and(schema::collection_items::multiple_collection_id.eq(collection_id)),
        )
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
    } else {
        collections_query = collections_query.limit(10)
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

pub fn delete_collection_by_id(
    user: &UserQuery,
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    id: i64,
) -> QueryResult<usize> {
    update(schema::multiple_collections::table)
        .filter(
            schema::multiple_collections::user_id
                .eq(user.id)
                .and(schema::multiple_collections::id.eq(id)),
        )
        .set(schema::multiple_collections::deleted_at.eq(chrono::offset::Utc::now().naive_utc()))
        .execute(pool)
}
