// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "subscription_type"))]
    pub struct SubscriptionType;
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::types::*;
    use super::sql_types::SubscriptionType;

    users (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        fxa_uid -> Varchar,
        fxa_refresh_token -> Varchar,
        avatar_url -> Nullable<Text>,
        is_subscriber -> Bool,
        subscription_type -> Nullable<SubscriptionType>,
    }
}
