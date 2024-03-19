use crate::api::common::{get_document_metadata, Sorting};
use crate::api::error::ApiError;
use crate::db::error::DbError;
use crate::db::model::UserQuery;
use crate::db::types::Subscription;
use crate::db::users::get_user;
use crate::db::v2::collection_items::{
    create_collection_item, delete_collection_item_in_collection, get_collection_item_by_id,
    multiple_collection_exists_for_user, update_collection_item,
};
use crate::db::v2::model::{CollectionItemAndDocumentQuery, MultipleCollectionsQuery};
use crate::db::v2::multiple_collections::{
    create_multiple_collection_for_user, edit_multiple_collection_for_user,
    get_collection_items_for_user_multiple_collection, get_collections_and_items_containing_url,
    get_count_of_multiple_collections_for_user, get_multiple_collection_by_id_for_user,
    get_multiple_collections_for_user, is_default_collection, multiple_collection_exists,
};
use crate::db::Pool;
use crate::helpers::to_utc;
use crate::ids::EncodedId;
use actix_identity::Identity;
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize)]
pub struct CollectionItemQueryParams {
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
    id: String,
    url: String,
    title: String,
    notes: Option<String>,
    parents: Vec<CollectionParent>,
    #[serde(serialize_with = "to_utc")]
    created_at: NaiveDateTime,
    #[serde(serialize_with = "to_utc")]
    updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct MultipleCollectionInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(serialize_with = "to_utc")]
    pub created_at: NaiveDateTime,
    #[serde(serialize_with = "to_utc")]
    pub updated_at: NaiveDateTime,
    pub article_count: i64,
}

#[derive(Serialize)]
pub struct MultipleCollectionResponse {
    #[serde(flatten)]
    pub info: MultipleCollectionInfo,
    pub items: Vec<CollectionItem>,
}

#[derive(Deserialize, Serialize, Validate, Clone)]
pub struct CollectionItemCreationRequest {
    #[validate(length(
        min = 1,
        max = 1024,
        message = "'title' must be between 1 and 1024 chars"
    ))]
    pub title: String,
    #[validate(length(min = 1, max = 1024))]
    pub url: String,
    #[validate(length(max = 65536, message = "'notes' must not be longer than 65536 chars"))]
    pub notes: Option<String>,
}

#[derive(Deserialize, Serialize, Validate, Clone)]
pub struct CollectionItemModificationRequest {
    #[validate(length(
        min = 1,
        max = 1024,
        message = "'title' must be between 1 and 1024 chars"
    ))]
    pub title: String,
    #[validate(length(max = 65536, message = "'notes' must not be longer than 65536 chars"))]
    pub notes: Option<String>,
}

#[derive(Deserialize, Serialize, Validate, Clone)]
pub struct MultipleCollectionCreationRequest {
    #[validate(length(
        min = 1,
        max = 1024,
        message = "'name' must be between 1 and 1024 chars"
    ))]
    pub name: String,
    #[validate(length(
        max = 65536,
        message = "'description' must not be longer than 65536 chars"
    ))]
    pub description: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MultipleCollectionLookupQueryParams {
    pub url: String,
}

#[derive(Serialize)]
pub struct LookupEntry {
    collection_id: String,
    item: CollectionItem,
}

#[derive(Serialize)]
pub struct MultipleCollectionLookupQueryResponse {
    results: Vec<LookupEntry>,
}

#[derive(Serialize)]
pub struct ConflictResponse {
    error: String,
}

pub struct CollectionAndItemId {
    pub collection_id: i64,
    pub item_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct EncodedCollectionAndItemId {
    pub collection_id: String,
    pub item_id: String,
}

impl TryFrom<&EncodedCollectionAndItemId> for CollectionAndItemId {
    type Error = ApiError;

    fn try_from(encoded: &EncodedCollectionAndItemId) -> Result<Self, Self::Error> {
        Ok(CollectionAndItemId {
            collection_id: EncodedId::decode(&encoded.collection_id)?,
            item_id: EncodedId::decode(&encoded.item_id)?,
        })
    }
}

impl From<&(i64, CollectionItemAndDocumentQuery)> for LookupEntry {
    fn from(val: &(i64, CollectionItemAndDocumentQuery)) -> Self {
        LookupEntry {
            collection_id: EncodedId::encode(val.0),
            item: val.1.to_owned().into(),
        }
    }
}

impl From<CollectionItemAndDocumentQuery> for CollectionItem {
    fn from(collection_and_document: CollectionItemAndDocumentQuery) -> Self {
        let mut parents: Option<Vec<CollectionParent>> = None;
        let mut title: Option<String> = None;
        let mut url = collection_and_document.uri;
        if let Some(metadata) = collection_and_document.metadata {
            parents = serde_json::from_value(metadata["parents"].clone()).unwrap_or(None);
            title = Some(match collection_and_document.custom_name {
                // We currently have empty strings instead of nulls due to our migration.
                // Let's fix this in the API for now.
                Some(custom_name) if !custom_name.is_empty() => custom_name,
                _ => collection_and_document.title,
            });
            url = serde_json::from_value(metadata["mdn_url"].clone()).unwrap_or(url);
        }
        CollectionItem {
            parents: parents.unwrap_or_default(),
            created_at: collection_and_document.created_at,
            updated_at: collection_and_document.updated_at,
            notes: collection_and_document.notes,
            url,
            title: title.unwrap_or_default(),
            id: EncodedId::encode(collection_and_document.id),
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
            id: EncodedId::encode(collection.id),
            article_count: collection.collection_item_count.unwrap_or(0),
        }
    }
}

pub async fn get_collections(
    _req: HttpRequest,
    user_id: Identity,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let res: Vec<MultipleCollectionInfo> =
        get_multiple_collections_for_user(&user, &mut conn_pool)?
            .into_iter()
            .map(MultipleCollectionInfo::from)
            .collect();
    Ok(HttpResponse::Ok().json(res))
}

pub async fn get_collection_by_id(
    user_id: Identity,
    pool: web::Data<Pool>,
    id: web::Path<EncodedId>,
    query: web::Query<CollectionItemQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let collection_id = id.into_inner();
    let collection_info =
        get_multiple_collection_by_id_for_user(&user, &mut conn_pool, &collection_id.get()?)?;
    if let Some(info) = collection_info {
        let collections_query = &query.into_inner();
        let res = get_collection_items_for_user_multiple_collection(
            &user,
            &mut conn_pool,
            &collection_id.get()?,
            collections_query,
        )?;
        let items = res.into_iter().map(Into::<CollectionItem>::into).collect();
        let collection_response = MultipleCollectionResponse {
            info: info.into(),
            items,
        };
        Ok(HttpResponse::Ok().json(collection_response))
    } else {
        Err(ApiError::CollectionNotFound(collection_id.id))
    }
}

pub async fn get_collection_item_in_collection_by_id(
    user_id: Identity,
    pool: web::Data<Pool>,
    path: web::Path<EncodedCollectionAndItemId>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let ids = &path.into_inner();
    let CollectionAndItemId {
        collection_id,
        item_id,
    } = ids.try_into()?;
    let collection_exists = multiple_collection_exists(&user, &collection_id, &mut conn_pool)?;
    if !collection_exists {
        return Err(ApiError::CollectionNotFound(ids.collection_id.to_owned()));
    }
    let res = get_collection_item_by_id(&user, &mut conn_pool, item_id)?;
    if let Some(item) = res {
        Ok(HttpResponse::Ok().json(CollectionItem::from(item)))
    } else {
        Err(ApiError::DocumentNotFound)
    }
}

pub async fn create_multiple_collection(
    pool: Data<Pool>,
    user_id: Identity,
    data: web::Json<MultipleCollectionCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    data.validate()?;
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id()?)?;
    let req = data.into_inner();
    let count = get_count_of_multiple_collections_for_user(&user, &mut conn_pool)?;
    let core_sub = Some(Subscription::Core) == user.get_subscription_type();
    if (count >= 3) && core_sub {
        return Err(ApiError::MultipleCollectionSubscriptionLimitReached);
    }
    let created = create_multiple_collection_for_user(&mut conn_pool, user.id, &req);

    match created {
        Err(db_err) => match db_err {
            DbError::Conflict(_) => Ok(HttpResponse::Conflict().json(ConflictResponse {
                error: format!("Collection with name '{}' already exists", &req.name),
            })),
            _ => Err(ApiError::DbError(db_err)),
        },
        Ok(result) => Ok(HttpResponse::Created().json(MultipleCollectionInfo::from(result))),
    }
}

pub async fn modify_collection(
    pool: Data<Pool>,
    user_id: Identity,
    collection_id: web::Path<EncodedId>,
    data: web::Json<MultipleCollectionCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    data.validate()?;
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id()?)?;
    let req = data.into_inner();
    let c_id = collection_id.into_inner();

    // REMOVE this once Migration to V2 is completed. Allow modification of notes but not name.
    if is_default_collection(&mut conn_pool, &user, c_id.get()?)? && req.name != "Default" {
        return Ok(HttpResponse::BadRequest().json(ConflictResponse {
            error: "Cannot modify default collection".to_owned(),
        }));
    }

    let updated = edit_multiple_collection_for_user(&mut conn_pool, user.id, c_id.get()?, &req);
    if let Err(db_err) = updated {
        match db_err {
            DbError::Conflict(_) => Ok(HttpResponse::Conflict().json(ConflictResponse {
                error: format!("Collection with name '{}' already exists", &req.name),
            })),
            DbError::NotFound(_) => Err(ApiError::CollectionNotFound(c_id.id)),
            _ => Err(ApiError::DbError(db_err)),
        }
    } else {
        Ok(HttpResponse::Ok().json(MultipleCollectionInfo::from(updated.unwrap())))
    }
}

pub async fn modify_collection_item_in_collection(
    pool: Data<Pool>,
    user_id: Identity,
    path: web::Path<EncodedCollectionAndItemId>,
    data: web::Json<CollectionItemModificationRequest>,
) -> Result<HttpResponse, ApiError> {
    data.validate()?;
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let ids = &path.into_inner();
    let CollectionAndItemId {
        collection_id,
        item_id,
    } = ids.try_into()?;
    let collection_exists = multiple_collection_exists(&user, &collection_id, &mut conn_pool)?;
    if !collection_exists {
        return Err(ApiError::CollectionNotFound(ids.collection_id.to_owned()));
    }
    update_collection_item(item_id, user.id, &mut conn_pool, &data.into_inner())?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn add_collection_item_to_collection(
    pool: Data<Pool>,
    http_client: Data<Client>,
    user_id: Identity,
    collection_id: web::Path<EncodedId>,
    data: web::Json<CollectionItemCreationRequest>,
) -> Result<HttpResponse, ApiError> {
    data.validate()?;
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let c_id = collection_id.into_inner();
    let collection_exists = multiple_collection_exists(&user, &c_id.get()?, &mut conn_pool)?;
    if !collection_exists {
        return Err(ApiError::CollectionNotFound(c_id.id));
    }
    let creation_data = data.into_inner();

    let metadata = get_document_metadata(http_client, &creation_data.url).await?;
    let res = create_collection_item(
        user.id,
        &mut conn_pool,
        &creation_data.url,
        metadata,
        &creation_data.to_owned(),
        c_id.get()?,
    );

    if let Err(db_err) = res {
        match db_err {
            DbError::Conflict(_) => Ok(HttpResponse::Conflict().json(ConflictResponse {
                error: "Collection item already exists in collection".to_string(),
            })),
            _ => Err(ApiError::DbError(db_err)),
        }
    } else {
        Ok(HttpResponse::Created().finish())
    }
}

pub async fn remove_collection_item_from_collection(
    pool: Data<Pool>,
    user_id: Identity,
    path: web::Path<EncodedCollectionAndItemId>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let ids = &path.into_inner();
    let CollectionAndItemId {
        collection_id,
        item_id,
    } = ids.try_into()?;
    if multiple_collection_exists_for_user(&user, &mut conn_pool, collection_id)? {
        delete_collection_item_in_collection(&user, &mut conn_pool, item_id)?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ApiError::CollectionNotFound(ids.collection_id.to_string()))
    }
}

pub async fn delete_collection(
    pool: Data<Pool>,
    user_id: Identity,
    collection_id: web::Path<EncodedId>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id()?)?;
    let collection_id = collection_id.into_inner().get()?;
    // REMOVE this once Migration to V2 is completed.
    if is_default_collection(&mut conn_pool, &user, collection_id)? {
        return Ok(HttpResponse::BadRequest().json(ConflictResponse {
            error: "Cannot delete default collection".to_owned(),
        }));
    }

    crate::db::v2::multiple_collections::delete_collection_by_id(
        &user,
        &mut conn_pool,
        collection_id,
    )?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn lookup_collections_containing_article(
    pool: Data<Pool>,
    user_id: Identity,
    page: web::Query<MultipleCollectionLookupQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id()?)?;
    let ids = get_collections_and_items_containing_url(&user, &mut conn_pool, page.url.as_str())?;
    let entries: Vec<LookupEntry> = ids.iter().map(|val| val.into()).collect();
    Ok(HttpResponse::Ok().json(MultipleCollectionLookupQueryResponse { results: entries }))
}
