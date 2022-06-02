use crate::db::schema::*;
use crate::db::types::Subscription;
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::Locale;

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

#[derive(Queryable, AsChangeset, Debug)]
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

#[derive(Queryable, Clone)]
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

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = collections)]
pub struct CollectionInsert {
    pub document_id: i64,
    pub custom_name: Option<String>,
    pub user_id: i64,
    pub notes: Option<String>,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = documents)]
pub struct DocumentInsert {
    pub absolute_uri: String,
    pub uri: String,
    pub metadata: Option<Value>,
    pub updated_at: NaiveDateTime,
    pub title: String,
}

#[derive(Queryable, Debug)]
#[diesel(table_name = settings)]
pub struct Settings {
    pub id: i64,
    pub user_id: i64,
    pub col_in_search: bool,
    pub locale_override: Option<Locale>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsQuery {
    pub col_in_search: Option<bool>,
    pub locale_override: Option<Option<Locale>>,
}

impl From<Settings> for SettingsQuery {
    fn from(val: Settings) -> Self {
        SettingsQuery {
            col_in_search: Some(val.col_in_search),
            locale_override: Some(val.locale_override),
        }
    }
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = settings)]
pub struct SettingsInsert {
    pub user_id: i64,
    pub col_in_search: Option<bool>,
    pub locale_override: Option<Option<Locale>>,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub mdn_url: String,
    pub parents: Option<Vec<CollectionParent>>,
    pub title: String,
}
