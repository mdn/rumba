#![allow(clippy::extra_unused_lifetimes)] /* https://github.com/rust-lang/rust-clippy/issues/9014 */
use crate::db::model::User;
use crate::db::schema::*;
use crate::db::types::BcdUpdateEventType;
use crate::helpers::{maybe_to_utc, to_utc};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::sql_types::{Date, Json, Text};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str;

#[derive(Queryable, Clone)]
pub struct CollectionItemAndDocumentQuery {
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

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = multiple_collections)]
pub struct MultipleCollectionInsert {
    pub deleted_at: Option<NaiveDateTime>,
    pub user_id: i64,
    pub notes: Option<String>,
    pub updated_at: NaiveDateTime,
    pub name: String,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = collection_items)]
pub struct CollectionItemInsert {
    pub document_id: i64,
    pub custom_name: Option<String>,
    pub user_id: i64,
    pub notes: Option<String>,
    pub multiple_collection_id: i64,
    pub updated_at: NaiveDateTime,
}

#[derive(Identifiable, Serialize, Queryable, Associations, PartialEq, Eq, Debug)]
#[diesel(belongs_to(User))]
#[diesel(table_name = multiple_collections)]
pub struct MultipleCollectionsQuery {
    pub id: i64,
    #[serde(serialize_with = "to_utc")]
    pub created_at: NaiveDateTime,
    #[serde(serialize_with = "to_utc")]
    pub updated_at: NaiveDateTime,
    #[serde(serialize_with = "maybe_to_utc")]
    pub deleted_at: Option<NaiveDateTime>,
    pub user_id: i64,
    pub notes: Option<String>,
    pub name: String,
    pub collection_item_count: Option<i64>,
}

impl From<MultipleCollectionsQueryNoCount> for MultipleCollectionsQuery {
    fn from(query: MultipleCollectionsQueryNoCount) -> Self {
        MultipleCollectionsQuery {
            id: query.id,
            created_at: query.created_at,
            updated_at: query.updated_at,
            deleted_at: query.deleted_at,
            user_id: query.user_id,
            notes: query.notes,
            name: query.name,
            collection_item_count: Some(0),
        }
    }
}

#[derive(Identifiable, Serialize, Queryable, Associations, PartialEq, Eq, Debug)]
#[diesel(belongs_to(User))]
#[diesel(table_name = multiple_collections)]
pub struct MultipleCollectionsQueryNoCount {
    pub id: i64,
    #[serde(serialize_with = "to_utc")]
    pub created_at: NaiveDateTime,
    #[serde(serialize_with = "to_utc")]
    pub updated_at: NaiveDateTime,
    #[serde(serialize_with = "maybe_to_utc")]
    pub deleted_at: Option<NaiveDateTime>,
    pub user_id: i64,
    pub notes: Option<String>,
    pub name: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct Events {
    pub events: Vec<Event>,
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
pub struct Event {
    pub path: String,
    pub mdn_url: Option<String>,
    pub source_file: Option<String>,
    pub spec_url: Option<String>,
    pub status: Option<Status>,
    pub event_type: BcdUpdateEventType,
}

#[derive(Clone, Eq, Debug, Deserialize, PartialEq, Serialize)]
pub struct Status {
    pub deprecated: bool,
    pub experimental: bool,
    pub standard_track: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Queryable, QueryableByName)]
pub struct BcdUpdateQuery {
    #[diesel(sql_type = Text)]
    pub browser: String,
    #[diesel(sql_type = Text)]
    pub browser_name: String,
    #[diesel(sql_type = Text)]
    pub engine: String,
    #[diesel(sql_type = Text)]
    pub engine_version: String,
    #[diesel(sql_type = Text)]
    pub release_id: String,
    #[diesel(sql_type = Date)]
    pub release_date: NaiveDate,
    #[diesel(sql_type = Json)]
    pub compat: Value,
}

pub struct BcdUpdate {
    pub browser: String,
    pub name: String,
    pub engine: String,
    pub engine_version: String,
    pub release_id: String,
    pub release_date: NaiveDate,
    pub compat: Vec<Event>,
}

impl From<&BcdUpdateQuery> for BcdUpdate {
    fn from(val: &BcdUpdateQuery) -> Self {
        BcdUpdate {
            browser: val.browser.clone(),
            engine: val.engine.clone(),
            name: val.browser_name.clone(),
            engine_version: val.engine_version.clone(),
            release_id: val.release_id.clone(),
            release_date: val.release_date,
            compat: serde_json::from_value::<Vec<Event>>(val.compat.clone()).unwrap(),
        }
    }
}
