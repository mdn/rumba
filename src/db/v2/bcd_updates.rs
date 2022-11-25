use crate::api::v2::multiple_collections::CollectionItemQueryParams;
use crate::api::v2::updates::BcdUpdatesQueryParams;
use crate::db::schema;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::{insert_into, PgConnection};
use r2d2::PooledConnection;

use crate::api::settings::SettingUpdateRequest;
use crate::db::error::DbError;
use crate::db::model::Settings;
use crate::db::model::SettingsInsert;
use crate::db::model::UserQuery;

use super::model::BcdUpdateQuery;

pub fn get_bcd_updates_paginated(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query_params: &BcdUpdatesQueryParams,
) -> Result<Vec<BcdUpdateQuery>, DbError> {
    let mut bcd_updates_query = schema::bcd_updates::table        
        .inner_join(schema::documents::table)
        .into_boxed();

    Ok(vec![])

    // if let Some(query) = &query_params.q {
    //     collections_query = collections_query.filter(
    //         schema::collection_items::custom_name
    //             .is_not_null()
    //             .and(
    //                 schema::collection_items::custom_name
    //                     .nullable()
    //                     .ilike(format!("%{}%", query)),
    //             )
    //             .or(schema::collection_items::custom_name
    //                 .is_null()
    //                 .and(schema::documents::title.ilike(format!("%{}%", query))))
    //             .or(schema::collection_items::notes
    //                 .nullable()
    //                 .ilike(format!("%{}%", query))),
    //     );
    // }

    // collections_query = match query_params.sort {
    //     Some(Sorting::Title) => collections_query
    //         .order_by(schema::collection_items::custom_name.desc())
    //         .then_order_by(schema::documents::title.desc()),
    //     Some(Sorting::Created) => {
    //         collections_query.order_by(schema::collection_items::created_at.desc())
    //     }
    //     None => collections_query.order_by(schema::collection_items::created_at.desc()),
    // };

    // if let Some(limit) = query_params.limit {
    //     collections_query = collections_query.limit(limit.into())
    // }

    // if let Some(offset) = query_params.offset {
    //     collections_query = collections_query.offset(offset.into())
    // }

    // Ok(collections_query
    //     .select((
    //         schema::collection_items::id,
    //         schema::collection_items::created_at,
    //         schema::collection_items::updated_at,
    //         schema::collection_items::document_id,
    //         schema::collection_items::notes,
    //         schema::collection_items::custom_name,
    //         schema::collection_items::user_id,
    //         schema::documents::uri,
    //         schema::documents::metadata,
    //         schema::documents::title,
    //     ))
    //     .get_results::<CollectionItemAndDocumentQuery>(pool)?)
}

pub fn create_or_update_settings(
    conn: &mut PgConnection,
    user: &UserQuery,
    settings_update: SettingUpdateRequest,
) -> QueryResult<usize> {
    let settings = SettingsInsert {
        user_id: user.id,
        col_in_search: settings_update.col_in_search,
        locale_override: settings_update.locale_override,
        multiple_collections: settings_update.multiple_collections,
    };
    insert_into(schema::settings::table)
        .values(&settings)
        .on_conflict(schema::settings::user_id)
        .do_update()
        .set(&settings)
        .execute(conn)
}
