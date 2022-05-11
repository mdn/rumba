use actix_identity::Identity;

use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::db::collections::{get_collection, get_collections_paginated};

use crate::db::Pool;

use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};

use chrono::NaiveDateTime;

use crate::db;
use crate::db::error::DbError;
use crate::db::model::{CollectionAndDocumentQuery, UserQuery};
use crate::db::users::get_user;

#[derive(Deserialize)]
pub struct CollectionsQueryParams {
    pub q: Option<String>,
    pub sort: Option<String>,
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
pub struct CollectionResponse {
    items: Vec<CollectionItem>,
    csrfmiddlewaretoken: String,
    subscription_limit_reached: bool,
}

impl From<CollectionAndDocumentQuery> for CollectionItem {
    fn from(collection_and_document: CollectionAndDocumentQuery) -> Self {
        let mut parents: Option<Vec<CollectionParent>> = None;
        let mut title: Option<String> = None;
        match collection_and_document.metadata {
            Some(metadata) => {
                parents = serde_json::from_value(metadata["parents"].clone()).unwrap_or(None);
                title = Some(
                    collection_and_document
                        .custom_name
                        .unwrap_or(collection_and_document.title),
                );
            }
            None => (),
        }
        CollectionItem {
            parents: parents.unwrap_or_default(),
            created: collection_and_document.created_at,
            notes: collection_and_document.notes,
            url: collection_and_document.uri,
            title: title.unwrap(),
            id: collection_and_document.id,
        }
    }
}

pub async fn collections(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    query: web::Query<CollectionsQueryParams>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            match &query.url {
                Some(url) => get_single_collection_item(pool, user, url).await,
                None => get_paginated_collection_items(pool, user, &query).await,
            }
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}

async fn get_single_collection_item(
    pool: web::Data<Pool>,
    user: UserQuery,
    url: &String,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let collection = get_collection(user, &mut conn, url).await;
    let items = match collection {
        Ok(val) => vec![val.into()],
        Err(e) => match e {
            DbError::DieselResult(_) => vec![],
            _ => return Err(ApiError::Unknown),
        },
    };
    let result = CollectionResponse {
        items,
        csrfmiddlewaretoken: "abc".to_string(),
        subscription_limit_reached: false,
    };
    Ok(HttpResponse::Ok().json(result))
}

async fn get_paginated_collection_items(
    pool: Data<Pool>,
    user: UserQuery,
    query: &CollectionsQueryParams,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let collection = get_collections_paginated(user, &mut conn, query)
        .await;
    
    let items = match collection {
        Ok(val) => val
            .iter()
            .map(|query_result| {
                Into::<CollectionItem>::into(query_result.clone())
            })
            .collect(),
        Err(e) => return Err(e.into()),
    };

    //##TODO Handle subscription limits

    let result = CollectionResponse {
        items,
        csrfmiddlewaretoken: "abc".to_string(),
        subscription_limit_reached: false,
    };
    Ok(HttpResponse::Ok().json(result))
}
