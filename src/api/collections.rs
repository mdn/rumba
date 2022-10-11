use actix_identity::Identity;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2::PooledConnection;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::error::ApiError;
use crate::db::collections::{
    collection_item_exists_for_user, create_collection_item, get_collection_item,
    get_collection_item_count, get_collection_items_paginated,
};

use crate::db::Pool;
use crate::settings::SETTINGS;

use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};

use chrono::NaiveDateTime;
use reqwest::Client;

use super::common::{get_document_metadata, Sorting};
use crate::db;
use crate::db::error::DbError;
use crate::db::model::{CollectionAndDocumentQuery, UserQuery};
use crate::db::users::get_user;
use crate::helpers::to_utc;

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
struct CollectionItem {
    id: i64,
    url: String,
    title: String,
    notes: Option<String>,
    parents: Vec<CollectionParent>,
    #[serde(serialize_with = "to_utc")]
    created: NaiveDateTime,
}

impl From<db::model::CollectionParent> for CollectionParent {
    fn from(parent: db::model::CollectionParent) -> Self {
        CollectionParent {
            uri: parent.uri,
            title: parent.title,
        }
    }
}

#[derive(Serialize)]
pub struct CollectionsResponse {
    items: Vec<CollectionItem>,
    csrfmiddlewaretoken: String,
    subscription_limit_reached: bool,
}

#[derive(Serialize)]
pub struct CollectionResponse {
    bookmarked: Option<CollectionItem>,
    csrfmiddlewaretoken: String,
    subscription_limit_reached: bool,
}

#[derive(Deserialize, Debug)]
pub struct CollectionCreationForm {
    pub name: String,
    pub notes: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CollectionCreationParams {
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct CollectionDeletionForm {
    pub delete: String,
}

#[derive(Deserialize, Debug)]
pub struct CollectionDeletionParams {
    pub url: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CollectionCreationOrDeletionForm {
    Deletion(CollectionDeletionForm),
    Creation(CollectionCreationForm),
}

impl From<CollectionAndDocumentQuery> for CollectionItem {
    fn from(collection_and_document: CollectionAndDocumentQuery) -> Self {
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
            created: collection_and_document.created_at,
            notes: collection_and_document.notes,
            url,
            title: title.unwrap_or_default(),
            id: collection_and_document.id,
        }
    }
}

pub async fn collections(
    _req: HttpRequest,
    user_id: Identity,
    pool: web::Data<Pool>,
    query: web::Query<CollectionsQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    match &query.url {
        Some(url) => get_single_collection_item(pool, user, url).await,
        None => get_paginated_collection_items(pool, &user, &query).await,
    }
}

async fn get_single_collection_item(
    pool: web::Data<Pool>,
    user: UserQuery,
    url: &str,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let collection = get_collection_item(&user, &mut conn_pool, url);
    let bookmarked = match collection {
        Ok(val) => Some(val.into()),
        Err(e) => match e {
            DbError::NotFound(_) => None,
            _ => return Err(ApiError::Unknown),
        },
    };

    let sub_info = collections_subscription_info_for_user(&user, &mut conn_pool).await?;

    let result = CollectionResponse {
        bookmarked,
        csrfmiddlewaretoken: "deprecated".to_string(),
        subscription_limit_reached: sub_info.limit_reached,
    };
    Ok(HttpResponse::Ok().json(result))
}

async fn get_paginated_collection_items(
    pool: Data<Pool>,
    user: &UserQuery,
    query: &CollectionsQueryParams,
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

    let sub_info = collections_subscription_info_for_user(user, &mut conn_pool).await?;

    let result = CollectionsResponse {
        items,
        csrfmiddlewaretoken: "deprecated".to_string(),
        subscription_limit_reached: sub_info.limit_reached,
    };
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create_or_update_collection_item(
    pool: Data<Pool>,
    http_client: Data<Client>,
    user_id: Identity,
    query: web::Query<CollectionCreationParams>,
    collection_form: web::Form<CollectionCreationOrDeletionForm>,
) -> Result<HttpResponse, ApiError> {
    match collection_form.into_inner() {
        CollectionCreationOrDeletionForm::Creation(collection_form) => {
            handle_create_update(
                &pool,
                user_id.id().unwrap(),
                query,
                http_client,
                collection_form,
            )
            .await
        }
        CollectionCreationOrDeletionForm::Deletion(collection_form)
            if collection_form.delete.to_lowercase() == "true" =>
        {
            delete_collection_item(
                pool,
                user_id,
                web::Query(CollectionDeletionParams {
                    url: query.into_inner().url,
                }),
            )
            .await
        }
        CollectionCreationOrDeletionForm::Deletion(collection_form)
            if collection_form.delete.to_lowercase() == "false" =>
        {
            undelete_collection_item(
                pool,
                user_id,
                web::Query(CollectionDeletionParams {
                    url: query.into_inner().url,
                }),
            )
            .await
        }
        CollectionCreationOrDeletionForm::Deletion(_) => Ok(HttpResponse::BadRequest().finish()),
    }
}

async fn handle_create_update(
    pool: &Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
    id: String,
    query: web::Query<CollectionCreationParams>,
    http_client: Data<Client>,
    collection_form: CollectionCreationForm,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, id)?;
    let url = &query.into_inner().url;
    let info = collections_subscription_info_for_user(&user, &mut conn_pool).await?;
    let collection_item_exists = collection_item_exists_for_user(&user, &mut conn_pool, url)?;
    if !collection_item_exists //Create or Update? 
        && info.collection_items_remaining.map_or(false, |val| val == 0)
    {
        return Ok(HttpResponse::BadRequest().json(json!({"error": "max_subscriptions", "info": { "max_allowed" : SETTINGS.application.subscriptions_limit_collections}})));
    }
    let metadata = get_document_metadata(http_client, url).await?;
    create_collection_item(
        &user,
        &mut conn_pool,
        url.clone(),
        metadata,
        collection_form,
    )
    .map_err(DbError::from)?;
    let subscription_limit_reached = info.collection_items_remaining.map_or(false, |val| {
        val - 1 <= SETTINGS.application.subscriptions_limit_collections
    });
    Ok(HttpResponse::Created().json(json!({
        "subscription_limit_reached": subscription_limit_reached
    })))
}

pub async fn undelete_collection_item(
    pool: Data<Pool>,
    user_id: Identity,
    query: web::Query<CollectionDeletionParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;

    let sub_info = collections_subscription_info_for_user(&user, &mut conn_pool).await?;
    if sub_info
        .collection_items_remaining
        .map_or(true, |number| number > 0)
    {
        let undeleted = crate::db::collections::undelete_collection_item(
            &user,
            &mut conn_pool,
            query.url.clone(),
        )
        .map_err(DbError::from)?
            == 1;
        let subscription_limit_reached =
            sub_info.collection_items_remaining.map_or(false, |number| {
                if undeleted {
                    // we successfully undeleted so number is off by 1
                    number < 2
                } else {
                    number < 1
                }
            });
        Ok(HttpResponse::Ok().json(json!({
            "subscription_limit_reached": subscription_limit_reached,
            "ok": true,
        })))
    } else {
        Ok(HttpResponse::Ok().json(json!({
            "subscription_limit_reached": true,
            "ok": false,
        })))
    }
}

pub async fn delete_collection_item(
    pool: Data<Pool>,
    user_id: Identity,
    query: web::Query<CollectionDeletionParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;

    let sub_info = collections_subscription_info_for_user(&user, &mut conn_pool).await?;
    crate::db::collections::delete_collection_item(&user, &mut conn_pool, query.url.clone())
        .map_err(DbError::from)?;

    let subscription_limit_reached = sub_info
        .collection_items_remaining
        .map_or(false, |number| number < 0);
    Ok(HttpResponse::Ok().json(json!({
        "subscription_limit_reached": subscription_limit_reached,
        "ok": true,
    })))
}

struct CollectionsSubscriptionInfo {
    pub limit_reached: bool,
    pub collection_items_remaining: Option<i64>,
}

async fn collections_subscription_info_for_user(
    user: &UserQuery,
    conn_pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<CollectionsSubscriptionInfo, ApiError> {
    match user.get_subscription_type() {
        Some(_type) => {
            if matches!(_type, db::types::Subscription::Core) {
                {
                    let count = get_collection_item_count(conn_pool, user.id)?;
                    let limit_reached =
                        count >= SETTINGS.application.subscriptions_limit_collections;

                    let watched_items_remaining =
                        SETTINGS.application.subscriptions_limit_collections - count;

                    Ok(CollectionsSubscriptionInfo {
                        limit_reached,
                        collection_items_remaining: Some(watched_items_remaining),
                    })
                }
            } else {
                Ok(CollectionsSubscriptionInfo {
                    limit_reached: false,
                    collection_items_remaining: None,
                })
            }
        }
        //Strange and impossible 'no subscription' case
        None => Ok(CollectionsSubscriptionInfo {
            limit_reached: true,
            collection_items_remaining: Some(0),
        }),
    }
}
