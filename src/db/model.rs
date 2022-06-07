use crate::db::types::{FxaEventStatus, Subscription};
use crate::db::{schema::*, types::FxaEvent};
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::Locale;
use super::types::NotificationTypeEnum;

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = users)]
pub struct User {
    pub updated_at: NaiveDateTime,
    pub fxa_uid: String,
    pub fxa_refresh_token: String,
    pub avatar_url: Option<String>,
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
    pub subscription_type: Option<Subscription>,
}

#[derive(Queryable, AsChangeset)]
#[diesel(table_name = users)]
pub struct UserUpdateFromWebhook {
    pub fxa_uid: String,
    pub email: Option<String>,
    pub subscription_type: Option<Option<Subscription>>,
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
    pub paths: Vec<String>,
}

#[derive(Queryable, Debug)]
#[diesel(table_name = settings)]
pub struct Settings {
    pub id: i64,
    pub user_id: i64,
    pub col_in_search: bool,
    pub locale_override: Option<Locale>,
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
    pub paths: Vec<String>,
}

#[derive(Queryable, Clone)]
pub struct NotificationsQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub starred: bool,
    pub read: bool,
    pub title: String,
    pub text: String,
    pub url: String,
}
#[derive(AsChangeset, Insertable)]
#[diesel(table_name = notifications)]
pub struct NotificationInsert {
    pub starred: bool,
    pub read: bool,
    pub deleted_at: Option<NaiveDateTime>,
    pub notification_data_id: i64,
    pub user_id: i64,
}

#[derive(Queryable, Clone)]
pub struct NotificationDataQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub text: String,
    pub url: String,
    pub data: Option<Value>,
    pub title: String,
    pub type_: NotificationTypeEnum,
    pub document_id: i64,
}

#[derive(AsChangeset, Insertable)]
#[diesel(table_name = notification_data)]
pub struct NotificationDataInsert {
    pub text: String,
    pub url: String,
    pub data: Option<Value>,
    pub title: String,
    pub type_: NotificationTypeEnum,
    pub document_id: i64,
}

#[derive(Queryable, Clone)]
pub struct WatchedItemsQuery {
    pub document_id: i64,
    pub user_id: i64,
    pub created_at: NaiveDateTime,
    pub uri: String,
    pub title: String,
    pub paths: Vec<Option<String>>,
}

#[derive(Insertable, AsChangeset, Clone)]
#[diesel(table_name = watched_items)]
pub struct WatchedItemInsert {
    pub document_id: i64,
    pub user_id: i64,
}

#[allow(dead_code)]
#[derive(Queryable)]
pub struct IdQuery {
    id: i64,
}

#[derive(Insertable, Queryable, AsChangeset)]
#[diesel(table_name = webhook_events)]
pub struct WebHooksEvent {
    pub fxa_uid: String,
    pub change_time: Option<NaiveDateTime>,
    pub issue_time: NaiveDateTime,
    pub typ: FxaEvent,
    pub status: FxaEventStatus,
    pub payload: Value,
}
