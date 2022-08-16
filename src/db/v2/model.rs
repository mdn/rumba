#![allow(clippy::extra_unused_lifetimes)] /* https://github.com/rust-lang/rust-clippy/issues/9014 */
use crate::db::model::User;
use crate::db::schema::*;
use chrono::NaiveDateTime;
use serde::Serialize;
use serde_json::Value;

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
}

#[derive(Identifiable, Serialize, Queryable, Associations, PartialEq, Eq, Debug)]
#[diesel(belongs_to(User))]
#[diesel(table_name = multiple_collections)]
pub struct MultipleCollectionsQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub user_id: i64,
    pub notes: Option<String>,
    pub name: String,
}
