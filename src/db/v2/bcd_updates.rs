use super::model::BcdUpdateQuery;
use crate::api::v2::updates::BcdUpdatesQueryParams;
use crate::db::error::DbError;

use crate::db::schema::*;
use diesel::dsl::*;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::sql_types::Jsonb;
use diesel::{PgConnection};
use r2d2::PooledConnection;
// pub fn get_bcd_updates_paginated(
//     pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
//     query_params: &BcdUpdatesQueryParams,
// ) -> Result<Vec<BcdUpdateQuery>, DbError> {
//     let paged = diesel::sql_query(
//             "select browser,
//             engine,
//             engine_version,
//             release_id,
//             release_date,
//             json_agg(json_build_object('event_type', bcd_updates.event_type,
//                                     'path', f.path,
//                                     'status', CASE
//                                                     WHEN (f.deprecated is null or f.standard_track is null or
//                                                         f.experimental is null) THEN null
//                                                     else json_build_object(
//                                                             'deprecated', f.deprecated,
//                                                             'standard_track', f.standard_track,
//                                                             'experimental', f.experimental
//                                                         ) END,
//                                     'mdn_url', f.mdn_url,
//                                     'source_file', f.source_file,
//                                     'spec_url', f.spec_url
//                 )) as compat
//     from bcd_updates
//             left join features f on bcd_updates.feature = f.id
//             left join browser_releases br on bcd_updates.browser_release = br.id
//     where bcd_updates.browser_release in (select id
//                                         from (SELECT DISTINCT browser,
//                                                                 release_date,
//                                                                 id,
//                                                                 release_id,
//                                                                 engine,
//                                                                 engine_version
//                                                 FROM browser_releases
//                                                 where release_date <= now()
//                                                 order by release_date desc, browser asc
//                                                 ) as last_ten)
//     group by 1, 2, 3, 4, 5
//     order by release_date desc, browser
//     limit $1 offset $2;                  
//     ");

//     Ok(paged
//         .bind::<Integer, _>(query_params.limit.unwrap_or(10))
//         .bind::<Integer, _>(query_params.offset.unwrap_or(0))
//         .get_results::<BcdUpdateQuery>(pool)?)
// }

pub fn get_bcd_updates_paginated(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> Result<Vec<BcdUpdateQuery>, DbError> {
    // #[diesel(sql_type = Text)]
    // pub browser: String,
    // #[diesel(sql_type = Text)]
    // pub engine: String,
    // #[diesel(sql_type = Text)]
    // pub engine_version: String,
    // #[diesel(sql_type = Text)]
    // pub release_id: String,
    // #[diesel(sql_type = Date)]
    // pub release_date: NaiveDate,
    // #[diesel(sql_type = Json)]
    // pub compat: Events,
// let res: Vec<BcdUpdateQuery> = 
let res =bcd_updates_view::table.group_by((bcd_updates_view::browser,
    bcd_updates_view::engine,
    bcd_updates_view::engine_version,
    bcd_updates_view::release_id,
    bcd_updates_view::release_date)).select((
        bcd_updates_view::browser,
        bcd_updates_view::engine,
        bcd_updates_view::engine_version,
        bcd_updates_view::release_id,
        bcd_updates_view::release_date,
        sql::<Jsonb>("json_agg(json_build_object('event_type', event_type,
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
    )) as compat")));
    // .load::<BcdUpdateQuery>(pool)?;

    // let dbg: diesel::query_builder::DebugQuery<_, _> = debug_query::<Pg,_>(&res);
    // info!("{:}",dbg);

    Ok(vec![])
}

#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
pub struct DateBrowser(
    browser_releases::id,
    browser_releases::browser,
    browser_releases::release_date,
);
