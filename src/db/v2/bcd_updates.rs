use super::model::BcdUpdate;
use super::model::BcdUpdateQuery;
use crate::api::v2::updates::BcdUpdatesQueryParams;
use crate::apply_filters;
use crate::bcd_updates_apply_sort;
use crate::bcd_updates_read_table_get_updates_for_collections;
use crate::bcd_updates_read_table_group_by_select;
use crate::db::error::DbError;
use crate::db::schema;
use crate::db::users::get_user;
use crate::db::v2::pagination::PaginationStats;
use crate::diesel::BoolExpressionMethods;
use crate::diesel::ExpressionMethods;
use crate::diesel::JoinOnDsl;
use crate::diesel::NullableExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;

use actix_identity::Identity;
use diesel::dsl::sql;

use diesel::r2d2::ConnectionManager;
use diesel::sql_types::Json;
use diesel::sql_types::{Nullable, Text};
use diesel::PgConnection;
use diesel::PgTextExpressionMethods;
use r2d2::PooledConnection;
sql_function!(fn lower(a: Nullable<Text>) -> Nullable<Text>);

const PAGE_LENGTH: i64 = 5;

fn offset_from_page(page: Option<i64>) -> i64 {
    match page {
        Some(page) if page > 0 => (page - 1) * PAGE_LENGTH,
        _ => 0,
    }
}

pub fn get_bcd_updates_paginated(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> Result<(Vec<BcdUpdate>, i64), DbError> {
    let count = get_count_for_query(pool, query_params);
    let mut query = bcd_updates_read_table_group_by_select!().into_boxed();
    query = apply_filters!(query, query_params, pool);
    query = bcd_updates_apply_sort!(query, query_params.sort);

    let offset = offset_from_page(query_params.page);

    let res = query
        .limit(PAGE_LENGTH)
        .offset(offset)
        .get_results::<BcdUpdateQuery>(pool)?;

    Ok((
        res.iter().map(BcdUpdate::from).collect(),
        count.ok().unwrap(),
    ))
}

pub fn get_bcd_updates_for_collection(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
    user_id: &Identity,
) -> Result<(Vec<BcdUpdate>, i64), DbError> {
    if let Some(collections) = &query_params.collections {
        let count = get_count_for_collections_query(
            pool,
            query_params,
            collections,
            &user_id.id().unwrap(),
        )?;

        let mut query = bcd_updates_read_table_get_updates_for_collections!(
            collections,
            &user_id.id().unwrap(),
            pool
        );

        query = apply_filters!(query, query_params, pool);
        query = bcd_updates_apply_sort!(query, query_params.sort);

        let offset = offset_from_page(query_params.page);

        let res = query
            .limit(PAGE_LENGTH)
            .offset(offset)
            .get_results::<BcdUpdateQuery>(pool)?;

        Ok((res.iter().map(BcdUpdate::from).collect(), count))
    } else {
        Ok((vec![], 0))
    }
}

pub fn get_count_for_query(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> Result<i64, DbError> {
    let mut query = bcd_updates_read_table_group_by_select!().into_boxed();
    query = apply_filters!(query, query_params, pool);
    let pages = query.paginate().per_page(5);
    Ok(pages.count_pages::<BcdUpdateQuery>(pool).unwrap())
}

pub fn get_count_for_collections_query(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
    collections: &Vec<i64>,
    user_id: &String,
) -> Result<i64, DbError> {
    let mut query = bcd_updates_read_table_get_updates_for_collections!(collections, user_id, pool);
    query = apply_filters!(query, query_params, pool);
    let pages = query.paginate().per_page(5);
    Ok(pages.count_pages::<BcdUpdateQuery>(pool).unwrap())
}
