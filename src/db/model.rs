use chrono::{NaiveDateTime};
use crate::db::types::Subscription;

use crate::db::schema::*;

#[derive(Insertable, AsChangeset)]
#[table_name = "users"]
pub struct User {
    pub updated_at: NaiveDateTime,
    pub fxa_uid: String,
    pub fxa_refresh_token: String,
    pub avatar_url: Option<String>,
    pub is_subscriber:  bool,
    pub subscription_type: Subscription,
}
