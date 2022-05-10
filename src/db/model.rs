use crate::db::schema::*;
use crate::db::types::Subscription;
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Queryable, AsChangeset)]
#[diesel(table_name = collections)]
#[diesel(belongs_to(User))]
pub struct CollectionQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub document_id: i64,
    pub notes: Option<String>,
    pub custom_name: Option<String>,
    pub user_id: i64,
}

#[derive(Queryable)]
pub struct CollectionAndDocumentQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub document_id: i64,
    pub notes: Option<String>,
    pub custom_name: Option<String>,
    pub user_id: i64,
    pub uri: String,
    pub metadata: Option<Value>,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionParent {
    pub uri: String,
    pub title: String,
}

#[derive(Queryable, AsChangeset)]
#[diesel(table_name = documents)]
pub struct DocumentQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub absolute_uri: String,
    pub uri: String,
    pub metadata: Option<Value>,
}

// id -> Int8,
// created_at -> Timestamp,
// updated_at -> Timestamp,
// absolute_uri -> Text,
// uri -> Text,
// metadata -> Nullable<Jsonb>,
// notes -> Nullable<Text>,
// custom_name -> Nullable<Text>,
