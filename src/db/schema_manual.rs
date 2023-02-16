use diesel::query_builder::QueryId;

use crate::db::schema::{sql_types::FxaEventType, *};

impl QueryId for FxaEventType {
    type QueryId = FxaEventType;

    const HAS_STATIC_QUERY_ID: bool = true;
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;
    use crate::db::schema::sql_types::BcdEventType;

    bcd_updates_view (browser, event_type, release_id, path) {
        browser_name -> Text,
        browser -> Text,
        category -> Text,
        deprecated -> Nullable<Bool>,
        description -> Nullable<Text>,
        engine -> Text,
        engine_version -> Text,
        event_type -> BcdEventType,
        experimental -> Nullable<Bool>,
        mdn_url -> Nullable<Text>,
        short_title -> Nullable<Text>,
        path -> Text,
        release_date -> Date,
        release_id -> Text,
        release_notes -> Nullable<Text>,
        source_file -> Text,
        spec_url -> Nullable<Text>,
        standard_track -> Nullable<Bool>,
        status -> Nullable<Text>,
        engines -> Array<Nullable<EngineType>>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(collection_items, bcd_updates_view,);

diesel::allow_tables_to_appear_in_same_query!(documents, bcd_updates_view,);
