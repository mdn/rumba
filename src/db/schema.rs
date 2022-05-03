table! {
    use diesel::sql_types::*;
    use crate::db::types::*;

    users (id) {
        id -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        email -> Text,
        fxa_uid -> Varchar,
        fxa_refresh_token -> Varchar,
        avatar_url -> Nullable<Text>,
        is_subscriber -> Bool,
        subscription_type -> Nullable<Subscription_type>,
    }
}
