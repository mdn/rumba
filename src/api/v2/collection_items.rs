use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2::PooledConnection;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::common::{get_document_metadata, Sorting};
use crate::api::error::ApiError;
use crate::api::user_middleware::UserId;

use crate::db::model::UserQuery;
use crate::db::v2::collection_items::{
    collection_item_exists_for_user, create_collection_item, get_collection_item,
    get_collection_items_paginated,
};
use crate::db::v2::model::CollectionItemAndDocumentQuery;
use crate::db::Pool;
use crate::settings::SETTINGS;

use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};

use chrono::NaiveDateTime;
use reqwest::Client;

use crate::db;
use crate::db::error::DbError;
use crate::db::users::get_user;

#[derive(Deserialize)]
pub struct CollectionItemQueryParams {
    pub q: Option<String>,
    pub sort: Option<Sorting>,
    pub url: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct CollectionItemParent {
    uri: String,
    title: String,
}

#[derive(Serialize)]
struct CollectionItem {
    id: i64,
    url: String,
    title: String,
    notes: Option<String>,
    parents: Vec<CollectionItemParent>,
    created: NaiveDateTime,
}

#[derive(Serialize)]
pub struct CollectionItemsResponse {
    items: Vec<CollectionItem>,
}

#[derive(Deserialize, Debug)]
pub struct CollectionItemCreationForm {
    pub name: String,
    pub notes: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CollectionItemCreationParams {
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct CollectionItemDeletionForm {
    pub delete: String,
}

#[derive(Deserialize, Debug)]
pub struct CollectionItemDeletionParams {
    pub url: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CollectionItemCreationOrDeletionForm {
    Deletion(CollectionItemDeletionForm),
    Creation(CollectionItemCreationForm),
}

impl From<CollectionItemAndDocumentQuery> for CollectionItem {
    fn from(collection_and_document: CollectionItemAndDocumentQuery) -> Self {
        let mut parents: Option<Vec<CollectionItemParent>> = None;
        let mut title: Option<String> = None;
        let mut url = collection_and_document.uri;
        match collection_and_document.metadata {
            Some(metadata) => {
                parents = serde_json::from_value(metadata["parents"].clone()).unwrap_or(None);
                title = Some(
                    collection_and_document
                        .custom_name
                        .unwrap_or(collection_and_document.title),
                );
                url = serde_json::from_value(metadata["mdn_url"].clone()).unwrap_or(url);
            }
            None => (),
        }
        CollectionItem {
            parents: parents.unwrap_or_default(),
            created: collection_and_document.created_at,
            notes: collection_and_document.notes,
            url,
            title: title.unwrap_or_default(),
            id: collection_and_document.id,
        }
    }
}

async fn get_paginated_collection_items(
    pool: Data<Pool>,
    user: &UserQuery,
    query: &CollectionItemQueryParams,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let collection = get_collection_items_paginated(user, &mut conn_pool, query);

    let items = match collection {
        Ok(val) => val
            .iter()
            .map(|query_result| Into::<CollectionItem>::into(query_result.clone()))
            .collect(),
        Err(e) => return Err(e.into()),
    };

    let result = CollectionItemsResponse { items };
    Ok(HttpResponse::Ok().json(result))
}
