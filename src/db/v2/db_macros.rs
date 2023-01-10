 #[macro_export]
macro_rules! bcd_updates_read_table_get_updates_for_collections {
    ($collections: expr, $user_id: expr, $conn_pool: expr) => {{

        let user_query: $crate::db::model::UserQuery = get_user($conn_pool, $user_id)?;
        let query = schema::bcd_updates_read_table::table
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
        .inner_join(
            schema::documents::table.on(schema::documents::uri.nullable().eq(lower(schema::bcd_updates_read_table::mdn_url)))
        )
        .inner_join(
            schema::collection_items::table.on(schema::documents::id.eq(schema::collection_items::document_id))
        )
        .filter(schema::collection_items::user_id.eq(user_query.id).and(schema::collection_items::multiple_collection_id.eq_any($collections)))
        .into_boxed();
        query
    }}
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
            )).into_boxed()
    }
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
        query
    }};
}
