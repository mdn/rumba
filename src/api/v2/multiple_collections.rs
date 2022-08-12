use std::fmt;

use crate::api::common::{get_document_metadata, Sorting};
use crate::api::error::ApiError;
use crate::api::user_middleware::UserId;
use crate::db;
use crate::db::error::DbError;
use crate::db::model::UserQuery;
use crate::db::users::get_user;
use crate::db::v2::collection_items::{
    collection_item_exists_for_user, create_collection_item, update_collection_item,
};
use crate::db::v2::model::{CollectionItemAndDocumentQuery, MultipleCollectionsQuery};
use crate::db::v2::multiple_collections::{
    add_collection_item_to_multiple_collection, create_multiple_collection_for_user,
    get_collection_item_id_for_collection, get_collection_items_for_user_multiple_collection,
    get_multiple_collection_by_id_for_user, get_multiple_collections_for_user,
    multiple_collection_exists,
};
use crate::db::Pool;
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;

use diesel::IntoSql;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::collection_items::{CollectionItemCreationForm, CollectionItemQueryParams};

#[derive(Deserialize)]
pub struct CollectionsQueryParams {
    pub q: Option<String>,
    pub sort: Option<Sorting>,
    pub url: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct CollectionParent {
    uri: String,
    title: String,
}

#[derive(Serialize)]
pub struct CollectionItem {
    id: i64,
    url: String,
    title: String,
    notes: Option<String>,
    parents: Vec<CollectionParent>,
    created: NaiveDateTime,
}

#[derive(Serialize)]
pub struct MultipleCollectionInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub article_count: i64,
}

#[derive(Serialize)]
pub struct MultipleCollectionResponse {
    #[serde(flatten)]
    pub info: MultipleCollectionInfo,
    pub items: Vec<CollectionItem>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectionItemCreationRequest {
    pub name: String,
    pub url: String,
    pub notes: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct MultipleCollectionCreationRequest {
    pub name: String,
    pub description: Option<String>,
}

impl From<CollectionItemCreationRequest> for CollectionItemCreationForm {
    fn from(val: CollectionItemCreationRequest) -> Self {
        CollectionItemCreationForm {
            name: val.name,
            notes: val.notes,
        }
    }
}

impl From<CollectionItemAndDocumentQuery> for CollectionItem {
    fn from(collection_and_document: CollectionItemAndDocumentQuery) -> Self {
        let mut parents: Option<Vec<CollectionParent>> = None;
        let mut title: Option<String> = None;
        let mut url = collection_and_document.uri;
        match collection_and_document.metadata {
            Some(metadata) => {
                parents = serde_json::from_value(metadata["parents"].clone()).unwrap_or(None);
                title = Some(match collection_and_document.custom_name {
                    // We currently have empty strings instead of nulls due to our migration.
                    // Let's fix this in the API for now.
                    Some(custom_name) if !custom_name.is_empty() => custom_name,
                    _ => collection_and_document.title,
                });
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

impl From<MultipleCollectionsQuery> for MultipleCollectionInfo {
    fn from(collection: MultipleCollectionsQuery) -> Self {
        MultipleCollectionInfo {
            name: collection.name,
            created_at: collection.created_at,
            updated_at: collection.updated_at,
            description: collection.notes,
            id: collection.id.to_string(),
            article_count: collection.collection_item_count.unwrap_or(0),
        }
    }
}

pub async fn get_collections(
    _req: HttpRequest,
    user_id: UserId,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    let res: Vec<MultipleCollectionInfo> =
        get_multiple_collections_for_user(&user, &mut conn_pool)?
            .into_iter()
            .map(|query| MultipleCollectionInfo::from(query))
            .collect();
    Ok(HttpResponse::Ok().json(res))
}

pub async fn get_collection_by_id(
    user_id: UserId,
    pool: web::Data<Pool>,
    id: web::Path<i64>,
    query: web::Query<CollectionItemQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    let collection_id = id.into_inner();
    let collection_info =
        get_multiple_collection_by_id_for_user(&user, &mut conn_pool, &collection_id)?;
    if let Some(info) = collection_info {
        let collections_query = &query.into_inner();
        let res = get_collection_items_for_user_multiple_collection(
            &user,
            &mut conn_pool,
            &collection_id,
            collections_query,
        )?;
        let items = res
            .iter()
            .map(|val| Into::<CollectionItem>::into(val.clone()))
            .collect();
        let collection_response = MultipleCollectionResponse {
            info: info.into(),
            items,
        };
        Ok(HttpResponse::Ok().json(collection_response))
    } else {
        return Ok(HttpResponse::BadRequest().finish());
    }
}

pub async fn create_multiple_collection(
    pool: Data<Pool>,
    user_id: UserId,
    data: web::Json<MultipleCollectionCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id)?;
    let req = data.into_inner();
    let created = create_multiple_collection_for_user(&mut conn_pool, user.id, &req);

    if let Err(db_err) = created {
        match db_err {
            DbError::Conflict(_) => {
                return Ok(HttpResponse::Conflict().json(json!({
                    "error": format!("Collection with name '{}' already exists", &req.name)
                })))
            }
            _ => return Err(ApiError::DbError(db_err)),
        }
    } else {
        Ok(HttpResponse::Created().json(MultipleCollectionInfo::from(created.unwrap())))
    }
}

pub async fn modify_collection_item_in_collection(
    pool: Data<Pool>,
    user_id: UserId,
    collection_id: web::Path<i64>,
    data: web::Json<CollectionItemCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(json!({"TODO" : true})))
}

pub async fn add_collection_item_to_collection(
    pool: Data<Pool>,
    http_client: Data<Client>,
    user_id: UserId,
    collection_id: web::Path<i64>,
    data: web::Json<CollectionItemCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;

    let user: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    let c_id = collection_id.into_inner();
    let collection_exists = multiple_collection_exists(&user, &c_id, &mut conn_pool)?;
    if !collection_exists {
        return Err(ApiError::CollectionNotFound);
    }
    let creation_data = data.into_inner();

    let collection_item_in_collection_id =
        get_collection_item_id_for_collection(&c_id, &user.id, &creation_data.url, &mut conn_pool)?;
    if let Some(ci_id) = collection_item_in_collection_id {
        info!("Found collection_item_id {} in Collection {}", ci_id, c_id);
        update_collection_item(ci_id, &mut conn_pool, &creation_data)?;
        return Ok(HttpResponse::Accepted().finish());
    } else {
        //Create new collection item.
        let metadata = get_document_metadata(http_client, &creation_data.url).await?;
        let created = create_collection_item(
            user.id,
            &mut conn_pool,
            &creation_data.url,
            metadata,
            &creation_data.to_owned().into(),
        )?;
        add_collection_item_to_multiple_collection(&mut conn_pool, &c_id, created)?;
        return Ok(HttpResponse::Created().finish());
    }
}

pub async fn remove_collection_item_from_collection(
    pool: Data<Pool>,
    http_client: Data<Client>,
    user_id: UserId,
    id: web::Path<i64>,
    query: web::Json<CollectionItemCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(json!({"TODO" : true})))

    // let mut conn_pool = pool.get()?;
    // let user: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    // let collection_id = id.into_inner();

    // Err(ApiError::DocumentNotFound)
}
