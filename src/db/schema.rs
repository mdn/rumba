// @generated automatically by Diesel CLI.

pub mod sql_types {
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
        is_subscriber -> Bool,
        subscription_type -> Nullable<SubscriptionType>,
    }
}

diesel::joinable!(collections -> documents (document_id));
diesel::joinable!(collections -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(collections, documents, users,);
