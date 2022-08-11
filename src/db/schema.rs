// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "fxa_event_status_type"))]
    pub struct FxaEventStatusType;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "fxa_event_type"))]
    pub struct FxaEventType;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "locale"))]
    pub struct Locale;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "notification_type"))]
    pub struct NotificationType;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "subscription_type"))]
    pub struct SubscriptionType;
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    collections (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
        document_id -> Int8,
        notes -> Nullable<Text>,
        custom_name -> Nullable<Text>,
        user_id -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    documents (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        absolute_uri -> Text,
        uri -> Text,
        metadata -> Nullable<Jsonb>,
        title -> Text,
        paths -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;
    use super::sql_types::NotificationType;

    notification_data (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        text -> Text,
        url -> Text,
        data -> Nullable<Jsonb>,
        title -> Text,
        #[sql_name = "type"]
        type_ -> NotificationType,
        document_id -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    notifications (id) {
        id -> Int8,
        user_id -> Int8,
        starred -> Bool,
        read -> Bool,
        deleted_at -> Nullable<Timestamp>,
        notification_data_id -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    raw_webhook_events_tokens (id) {
        id -> Int8,
        received_at -> Timestamp,
        token -> Text,
        error -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;
    use super::sql_types::Locale;

    settings (id) {
        id -> Int8,
        user_id -> Int8,
        col_in_search -> Bool,
        locale_override -> Nullable<Locale>,
        multiple_collections -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;
    use super::sql_types::SubscriptionType;

    users (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        email -> Text,
        fxa_uid -> Varchar,
        fxa_refresh_token -> Varchar,
        avatar_url -> Nullable<Text>,
        subscription_type -> Nullable<SubscriptionType>,
        enforce_plus -> Nullable<SubscriptionType>,
        is_admin -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    watched_items (user_id, document_id) {
        user_id -> Int8,
        document_id -> Int8,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;
    use super::sql_types::FxaEventType;
    use super::sql_types::FxaEventStatusType;

    webhook_events (id) {
        id -> Int8,
        fxa_uid -> Varchar,
        change_time -> Nullable<Timestamp>,
        issue_time -> Timestamp,
        typ -> FxaEventType,
        status -> FxaEventStatusType,
        payload -> Jsonb,
    }
}

diesel::joinable!(collections -> documents (document_id));
diesel::joinable!(collections -> users (user_id));
diesel::joinable!(notification_data -> documents (document_id));
diesel::joinable!(notifications -> notification_data (notification_data_id));
diesel::joinable!(notifications -> users (user_id));
diesel::joinable!(settings -> users (user_id));
diesel::joinable!(watched_items -> documents (document_id));
diesel::joinable!(watched_items -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    collections,
    documents,
    notification_data,
    notifications,
    raw_webhook_events_tokens,
    settings,
    users,
    watched_items,
    webhook_events,
);
