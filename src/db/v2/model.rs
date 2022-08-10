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
#[diesel(table_name = collection_items)]
pub struct CollectionItemInsert {
    pub document_id: i64,
    pub custom_name: Option<String>,
    pub user_id: i64,
    pub notes: Option<String>,
}

#[derive(Identifiable, Serialize, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(User)]
#[table_name = "multiple_collections"]
pub struct MultipleCollectionsQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub user_id: i64,
    pub notes: Option<String>,
    pub name: String,
    pub collection_item_count: i64,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = multiple_collections_to_items)]
pub struct CollectionToItemsInsert {
    pub multiple_collection_id: i64,
    pub collection_item_id: i64,
}
