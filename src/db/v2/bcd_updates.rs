use super::model::BcdUpdate;
use super::model::BcdUpdateQuery;
use crate::api::v2::updates::BcdUpdatesQueryParams;
use crate::apply_filters;
use crate::bcd_updates_read_table_group_by_select;
use crate::db::error::DbError;
use crate::db::schema;
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use diesel::dsl::sql;

use crate::db::v2::pagination::PaginationStats;
use crate::diesel::PgTextExpressionMethods;
use diesel::r2d2::ConnectionManager;
use diesel::sql_types::Json;
use diesel::PgConnection;
use r2d2::PooledConnection;
pub fn get_bcd_updates_paginated(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> Result<(Vec<BcdUpdate>, i64), DbError> {
    let count = get_count_for_query(pool, query_params);

    let mut query = bcd_updates_read_table_group_by_select!();
    query = apply_filters!(query, query_params);

    let offset = (query_params.page.map_or(1, |val| {
        if val <= 0 {
            return 1;
        }
        val
    }) - 1)
        * 5;

    if let Some(sort) = &query_params.sort {
        match sort {
            crate::api::v2::updates::AscOrDesc::Asc => {
                query = query.order_by((
                    schema::bcd_updates_read_table::release_date.asc(),
                    schema::bcd_updates_read_table::browser_name,
                ))
            }
            crate::api::v2::updates::AscOrDesc::Desc => {
                query = query.order_by((
                    schema::bcd_updates_read_table::release_date.desc(),
                    schema::bcd_updates_read_table::browser_name,
                ))
            }
        }
    }

    let res = query
        .limit(5)
        .offset(offset)
        .get_results::<BcdUpdateQuery>(pool)?;

    Ok((res.iter().map(BcdUpdate::from).collect(), count))
}

pub fn get_count_for_query(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> i64 {
    let mut query = bcd_updates_read_table_group_by_select!();
    query = apply_filters!(query, query_params);
    let pags = query.paginate().per_page(5);
    pags.count_pages::<BcdUpdateQuery>(pool).unwrap()
}
