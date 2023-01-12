#[macro_export]
macro_rules! bcd_updates_apply_sort {
    ($query: expr, $sort: expr) => {
        if let Some(sort) = &$sort {
            match sort {
                $crate::api::v2::updates::AscOrDesc::Asc => $query.order_by((
                    schema::bcd_updates_read_table::release_date.asc(),
                    schema::bcd_updates_read_table::browser_name,
                )),
                $crate::api::v2::updates::AscOrDesc::Desc => $query.order_by((
                    schema::bcd_updates_read_table::release_date.desc(),
                    schema::bcd_updates_read_table::browser_name,
                )),
            }
        } else {
            $query.order_by((
                schema::bcd_updates_read_table::release_date.desc(),
                schema::bcd_updates_read_table::browser_name,
            ))
        }
    };
}

#[macro_export]
macro_rules! bcd_updates_read_table_group_by_select {
    () => {
schema::bcd_updates_read_table::table
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
        ))
    }
}

#[macro_export]
macro_rules! bcd_updates_read_table_get_updates_for_collections {
    ($collections: expr, $user_id: expr, $conn_pool: expr) => {{
        let user_query: $crate::db::model::UserQuery = get_user($conn_pool, $user_id)?;
        let query = $crate::bcd_updates_read_table_group_by_select!()
            .inner_join(
                schema::documents::table.on(schema::documents::uri
                    .nullable()
                    .eq(lower(schema::bcd_updates_read_table::mdn_url))),
            )
            .inner_join(
                schema::collection_items::table
                    .on(schema::documents::id.eq(schema::collection_items::document_id)),
            )
            .filter(
                schema::collection_items::user_id
                    .eq(user_query.id)
                    .and(schema::collection_items::multiple_collection_id.eq_any($collections)),
            )
            .into_boxed();
        query
    }};
}

#[macro_export]
macro_rules! apply_filters {
    ($query: expr, $query_params: expr, $user_id: expr, $conn_pool: expr) => {{
        let mut query = $query;

        if let Some(search) = &$query_params.q {
            query =
                query.filter(schema::bcd_updates_read_table::path.ilike(format!("%{:}%", search)));
        }

        if let Some(category) = &$query_params.category {
            query = query.filter(schema::bcd_updates_read_table::category.eq_any(category));
        }

        if let Some(browsers) = &$query_params.browsers {
            query = query.filter(schema::bcd_updates_read_table::browser.eq_any(browsers));
        }

        if let (Some(show), Some(user)) = (&$query_params.show, $user_id) {
            if show.eq("watched") {
                let user_db_id = get_user($conn_pool, &user)?;
                let watched_pages =
                    get_watched_items($conn_pool, user_db_id.id, &Default::default())?;
                let user_uris: Vec<String> = watched_pages
                    .iter()
                    .map(|query| query.uri.to_owned())
                    .collect();
                query =
                    query.filter(lower(schema::bcd_updates_read_table::mdn_url).eq_any(user_uris));
            }
        }
        query
    }};
}
