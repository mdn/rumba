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

    auth_group (id) {
        id -> Int4,
        name -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    auth_group_permissions (id) {
        id -> Int8,
        group_id -> Int4,
        permission_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    auth_permission (id) {
        id -> Int4,
        name -> Varchar,
        content_type_id -> Int4,
        codename -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    auth_user (id) {
        id -> Int4,
        password -> Varchar,
        last_login -> Nullable<Timestamptz>,
        is_superuser -> Bool,
        username -> Varchar,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Varchar,
        is_staff -> Bool,
        is_active -> Bool,
        date_joined -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    auth_user_groups (id) {
        id -> Int8,
        user_id -> Int4,
        group_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    auth_user_user_permissions (id) {
        id -> Int8,
        user_id -> Int4,
        permission_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    bookmarks_bookmark (id) {
        id -> Int8,
        deleted -> Nullable<Timestamptz>,
        created -> Timestamptz,
        modified -> Timestamptz,
        documenturl_id -> Int8,
        user_id -> Int4,
        custom_name -> Varchar,
        notes -> Varchar,
    }
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

    django_admin_log (id) {
        id -> Int4,
        action_time -> Timestamptz,
        object_id -> Nullable<Text>,
        object_repr -> Varchar,
        action_flag -> Int2,
        change_message -> Text,
        content_type_id -> Nullable<Int4>,
        user_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    django_content_type (id) {
        id -> Int4,
        app_label -> Varchar,
        model -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    django_migrations (id) {
        id -> Int8,
        app -> Varchar,
        name -> Varchar,
        applied -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    django_session (session_key) {
        session_key -> Varchar,
        session_data -> Text,
        expire_date -> Timestamptz,
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

    documenturls_documenturl (id) {
        id -> Int8,
        uri -> Varchar,
        absolute_url -> Varchar,
        metadata -> Nullable<Jsonb>,
        invalid -> Nullable<Timestamptz>,
        created -> Timestamptz,
        modified -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    documenturls_documenturlcheck (id) {
        id -> Int8,
        http_error -> Int4,
        headers -> Jsonb,
        created -> Timestamptz,
        document_url_id -> Int8,
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

    notifications_defaultwatch (id) {
        id -> Int8,
        content_updates -> Bool,
        browser_compatibility -> Jsonb,
        user_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    notifications_notification (id) {
        id -> Int8,
        read -> Bool,
        notification_id -> Int8,
        user_id -> Int4,
        starred -> Bool,
        deleted -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    notifications_notificationdata (id) {
        id -> Int8,
        title -> Varchar,
        text -> Varchar,
        created -> Timestamptz,
        modified -> Timestamptz,
        data -> Jsonb,
        #[sql_name = "type"]
        type_ -> Varchar,
        page_url -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    notifications_watch (id) {
        id -> Int8,
        path -> Varchar,
        url -> Text,
        title -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    notifications_watch_users (id) {
        id -> Int8,
        user_id -> Int4,
        watch_id -> Int8,
        browser_compatibility -> Jsonb,
        content_updates -> Bool,
        custom -> Bool,
        custom_default -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    plus_landingpagesurvey (uuid) {
        uuid -> Uuid,
        response -> Nullable<Jsonb>,
        geo_information -> Nullable<Text>,
        created -> Timestamptz,
        updated -> Timestamptz,
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
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    users_accountevent (id) {
        id -> Int8,
        created_at -> Timestamptz,
        modified_at -> Timestamptz,
        fxa_uid -> Varchar,
        payload -> Text,
        event_type -> Int4,
        status -> Int4,
        jwt_id -> Varchar,
        issued_at -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    users_userprofile (id) {
        id -> Int8,
        locale -> Nullable<Varchar>,
        created -> Timestamptz,
        modified -> Timestamptz,
        user_id -> Int4,
        avatar -> Varchar,
        fxa_refresh_token -> Varchar,
        is_subscriber -> Bool,
        subscription_type -> Varchar,
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

diesel::joinable!(auth_group_permissions -> auth_group (group_id));
diesel::joinable!(auth_group_permissions -> auth_permission (permission_id));
diesel::joinable!(auth_permission -> django_content_type (content_type_id));
diesel::joinable!(auth_user_groups -> auth_group (group_id));
diesel::joinable!(auth_user_groups -> auth_user (user_id));
diesel::joinable!(auth_user_user_permissions -> auth_permission (permission_id));
diesel::joinable!(auth_user_user_permissions -> auth_user (user_id));
diesel::joinable!(bookmarks_bookmark -> auth_user (user_id));
diesel::joinable!(bookmarks_bookmark -> documenturls_documenturl (documenturl_id));
diesel::joinable!(collections -> documents (document_id));
diesel::joinable!(collections -> users (user_id));
diesel::joinable!(django_admin_log -> auth_user (user_id));
diesel::joinable!(django_admin_log -> django_content_type (content_type_id));
diesel::joinable!(documenturls_documenturlcheck -> documenturls_documenturl (document_url_id));
diesel::joinable!(notification_data -> documents (document_id));
diesel::joinable!(notifications -> notification_data (notification_data_id));
diesel::joinable!(notifications -> users (user_id));
diesel::joinable!(notifications_defaultwatch -> auth_user (user_id));
diesel::joinable!(notifications_notification -> auth_user (user_id));
diesel::joinable!(notifications_notification -> notifications_notificationdata (notification_id));
diesel::joinable!(notifications_watch_users -> auth_user (user_id));
diesel::joinable!(notifications_watch_users -> notifications_watch (watch_id));
diesel::joinable!(settings -> users (user_id));
diesel::joinable!(users_userprofile -> auth_user (user_id));
diesel::joinable!(watched_items -> documents (document_id));
diesel::joinable!(watched_items -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    auth_group,
    auth_group_permissions,
    auth_permission,
    auth_user,
    auth_user_groups,
    auth_user_user_permissions,
    bookmarks_bookmark,
    collections,
    django_admin_log,
    django_content_type,
    django_migrations,
    django_session,
    documents,
    documenturls_documenturl,
    documenturls_documenturlcheck,
    notification_data,
    notifications,
    notifications_defaultwatch,
    notifications_notification,
    notifications_notificationdata,
    notifications_watch,
    notifications_watch_users,
    plus_landingpagesurvey,
    raw_webhook_events_tokens,
    settings,
    users,
    users_accountevent,
    users_userprofile,
    watched_items,
    webhook_events,
);
