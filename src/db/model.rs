use crate::db::types::Subscription;
use chrono::NaiveDateTime;

use crate::db::schema::*;

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = users)]
pub struct User {
    pub updated_at: NaiveDateTime,
    pub fxa_uid: String,
    pub fxa_refresh_token: String,
    pub avatar_url: Option<String>,
    pub is_subscriber: bool,
    pub subscription_type: Subscription,
    pub email: String,
}

#[derive(Queryable, AsChangeset)]
#[diesel(table_name = users)]
pub struct UserQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub email: String,
    pub fxa_uid: String,
    pub fxa_refresh_token: String,
    pub avatar_url: Option<String>,
    pub is_subscriber: bool,
    pub subscription_type: Option<Subscription>,
}
