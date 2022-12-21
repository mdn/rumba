use super::model::BcdUpdate;
use super::model::BcdUpdateQuery;
use crate::api::v2::updates::BcdUpdatesQueryParams;
use crate::apply_filters;
use crate::bcd_updates_read_table_group_by_select;
use crate::db::error::DbError;
use crate::db::schema;
use crate::db::users::get_user;
use crate::db::watched_items::get_watched_items;
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use actix_identity::Identity;
use diesel::dsl::sql;

use crate::db::v2::pagination::PaginationStats;

use crate::diesel::PgTextExpressionMethods;
use diesel::r2d2::ConnectionManager;
use diesel::sql_types::Json;
use diesel::sql_types::{Nullable, Text};
use diesel::PgConnection;
use r2d2::PooledConnection;
sql_function!(fn lower(a: Nullable<Text>) -> Nullable<Text>);

pub fn get_bcd_updates_paginated(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
    user_id: Option<Identity>,
) -> Result<(Vec<BcdUpdate>, i64), DbError> {
    let count = get_count_for_query(pool, query_params, &user_id);

    let mut query = bcd_updates_read_table_group_by_select!();
    query = apply_filters!(query, query_params, user_id, pool);

    if let (Some(show), Some(user)) = (&query_params.show, user_id) {
        if show.eq("watched") {
            let id = user.id().unwrap();
            let user_db_id = get_user(pool, id)?;
            let watched_pages = get_watched_items(pool, user_db_id.id, &Default::default())?;
            let user_uris: Vec<String> = watched_pages
                .iter()
                .map(|query| query.uri.to_owned())
                .collect();
            query = query.filter(lower(schema::bcd_updates_read_table::mdn_url).eq_any(user_uris));
        }
    }

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
    } else {
        query = query.order_by((
            schema::bcd_updates_read_table::release_date.desc(),
            schema::bcd_updates_read_table::browser_name,
        ))
    }

    let res = query
        .limit(5)
        .offset(offset)
        .get_results::<BcdUpdateQuery>(pool)?;

    Ok((
        res.iter().map(BcdUpdate::from).collect(),
        count.ok().unwrap(),
    ))
}

pub fn get_count_for_query(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
    user_id: &Option<Identity>,
) -> Result<i64, DbError> {
    let mut query = bcd_updates_read_table_group_by_select!();
    query = apply_filters!(query, query_params, user_id, pool);
    if let (Some(show), Some(user)) = (&query_params.show, user_id) {
        if show.eq("watched") {
            let id = user.id().unwrap();
            let user_db_id = get_user(pool, id)?;
            let watched_pages = get_watched_items(pool, user_db_id.id, &Default::default())?;
            let user_uris: Vec<String> = watched_pages
                .iter()
                .map(|query| query.uri.to_owned())
                .collect();
            query = query.filter(lower(schema::bcd_updates_read_table::mdn_url).eq_any(user_uris));
        }
    }
    let pags = query.paginate().per_page(5);
    Ok(pags.count_pages::<BcdUpdateQuery>(pool).unwrap())
}
