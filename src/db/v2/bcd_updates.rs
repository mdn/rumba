use super::model::BcdUpdate;
use super::model::BcdUpdateQuery;
use crate::api::v2::updates::BcdUpdatesQueryParams;
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

    let mut query = schema::bcd_updates_read_table::table
        .group_by((
            schema::bcd_updates_read_table::browser,
            schema::bcd_updates_read_table::browser_name,
            schema::bcd_updates_read_table::engine,
            schema::bcd_updates_read_table::engine_version,
            schema::bcd_updates_read_table::release_id,
            schema::bcd_updates_read_table::release_date
        ))
        .select((
            schema::bcd_updates_read_table::browser,
            schema::bcd_updates_read_table::browser_name,
            schema::bcd_updates_read_table::engine,
            schema::bcd_updates_read_table::engine_version,
            schema::bcd_updates_read_table::release_id,
            schema::bcd_updates_read_table::release_date,
            sql::<Json>(
                "json_agg(json_build_object('event_type', event_type,
                                            'path', path,
                                            'status', CASE
                                                        WHEN (deprecated is null or standard_track is null or
                                                                experimental is null) THEN null
                                                        else json_build_object(
                                                                'deprecated', deprecated,
                                                                'standard_track', standard_track,
                                                                'experimental', experimental
                                                            ) END,
                                            'mdn_url', mdn_url,
                                            'source_file', source_file,
                                            'spec_url', spec_url
     )) as compat",
            ),
        )).into_boxed();

    if let Some(search) = &query_params.q {
        query = query.filter(schema::bcd_updates_read_table::path.ilike(format!("%{:}%", search)));
    }

    if let Some(since) = &query_params.live_since {
        query = query.filter(schema::bcd_updates_read_table::release_date.ge(since));
    }

    if let Some(browsers) = &query_params.browsers {
        query = query.filter(schema::bcd_updates_read_table::browser.eq_any(browsers));
    }

    let offset = (query_params.page.map_or(1, |val| {
        if val <= 0 {
            return 1;
        }
        val
    }) - 1)
        * 5;

    let res = query
        .order_by((
            schema::bcd_updates_read_table::release_date.desc(),
            schema::bcd_updates_read_table::browser_name,
        ))
        .limit(5)
        .offset(offset)
        .get_results::<BcdUpdateQuery>(pool)?;

    Ok((res.iter().map(BcdUpdate::from).collect(), count))
}

/*
  Massive #TODO. Figure out how to get this non-duplicated.
*/

pub fn get_count_for_query(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> i64 {
    let mut query = schema::bcd_updates_read_table::table
    .group_by((
        schema::bcd_updates_read_table::browser,
        schema::bcd_updates_read_table::browser_name,
schema::bcd_updates_read_table::engine,
schema::bcd_updates_read_table::engine_version,
schema::bcd_updates_read_table::release_id,
schema::bcd_updates_read_table::release_date))
    .select((
        schema::bcd_updates_read_table::browser,
        schema::bcd_updates_read_table::browser_name,
        schema::bcd_updates_read_table::engine,
        schema::bcd_updates_read_table::engine_version,
        schema::bcd_updates_read_table::release_id,
        schema::bcd_updates_read_table::release_date,
        sql::<Json>(
            "json_agg(json_build_object('event_type', event_type,
                                        'path', path,
                                        'status', CASE
                                                    WHEN (deprecated is null or standard_track is null or
                                                            experimental is null) THEN null
                                                    else json_build_object(
                                                            'deprecated', deprecated,
                                                            'standard_track', standard_track,
                                                            'experimental', experimental
                                                        ) END,
                                        'mdn_url', mdn_url,
                                        'source_file', source_file,
                                        'spec_url', spec_url
 )) as compat",
        ),
    )).into_boxed();

    if let Some(search) = &query_params.q {
        query = query.filter(schema::bcd_updates_read_table::path.ilike(format!("%{:}%", search)))
    }

    if let Some(since) = &query_params.live_since {
        query = query.filter(schema::bcd_updates_read_table::release_date.ge(since));
    }

    if let Some(browsers) = &query_params.browsers {
        query = query.filter(schema::bcd_updates_read_table::browser.eq_any(browsers));
    }

    let pags = query.paginate().per_page(5);
    pags.count_pages::<BcdUpdateQuery>(pool).unwrap()
}
