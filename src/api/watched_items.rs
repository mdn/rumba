use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2::PooledConnection;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        model::{UserQuery, WatchedItemsQuery},
        users::get_user,
        watched_items::{self, create_watched_item, delete_watched_item},
        Pool,
    },
    util::normalize_uri,
};

use super::{
    common::{get_document_metadata, Sorting},
    error::ApiError,
};

#[derive(Deserialize)]
pub struct WatchedItemQueryParams {
    pub url: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub q: Option<String>,
    pub sort: Option<Sorting>,
}

#[derive(Deserialize)]
pub struct UpdateWatchedItemFormData {
    pub unwatch: Option<bool>,
    pub title: Option<String>,
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateWatchedItemQueryParams {
    pub url: String,
}

#[derive(Serialize)]
struct WatchedItemsResponse {
    pub items: Vec<WatchedItem>,
    pub csrfmiddlewaretoken: String,
    /* Status will always be 'major' or 'unwatched' for back compat */
}

#[derive(Serialize)]
struct SingleWatchedItemResponse {
    #[serde(flatten)]
    pub result: WatchedItem,
    pub csrfmiddlewaretoken: String,
    /* Status will always be 'major' or 'unwatched' for back compat */
}

#[derive(Serialize)]
struct EmptyWatchedItemResponse {
    pub status: String,
    pub csrfmiddlewaretoken: String,
    /* Status will always be 'major' or 'unwatched' for back compat */
}

#[derive(Serialize)]
pub struct WatchedItem {
    title: String,
    url: String,
    path: String,
    status: String,
}

impl From<WatchedItemsQuery> for WatchedItem {
    fn from(watched_item: WatchedItemsQuery) -> Self {
        let path = watched_item
            .paths
            .first()
            .unwrap_or(&Some(watched_item.uri.clone()))
            .to_owned()
            .unwrap_or_else(|| watched_item.uri.clone());

        WatchedItem {
            title: watched_item.title,
            url: watched_item.uri,
            path,
            status: "major".to_string(),
        }
    }
}

pub async fn get_watched_items(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    query: web::Query<WatchedItemQueryParams>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            let query = query.into_inner();
            if let Some(url) = query.url {
                handle_single_item_query(&mut conn_pool, &user, &url).await
            } else {
                handle_paginated_items_query(&mut conn_pool, user, &query).await
            }
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}

async fn handle_paginated_items_query(
    conn_pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: UserQuery,
    query: &WatchedItemQueryParams,
) -> Result<HttpResponse, ApiError> {
    let res = watched_items::get_watched_items(conn_pool, user.id, query).await?;
    let items: Vec<WatchedItem> = res
        .iter()
        .map(|watched_item| Into::<WatchedItem>::into(watched_item.clone()))
        .collect();
    Ok(HttpResponse::Ok().json(WatchedItemsResponse {
        items,
        csrfmiddlewaretoken: "TODO".to_string(),
    }))
}

async fn handle_single_item_query(
    conn_pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: &UserQuery,
    url: &str,
) -> Result<HttpResponse, ApiError> {
    let res = watched_items::get_watched_item(conn_pool, user.id, &normalize_uri(url)).await?;

    if let Some(item) = res {
        Ok(HttpResponse::Ok().json(SingleWatchedItemResponse {
            result: item.into(),
            csrfmiddlewaretoken: "TODO".to_string(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(EmptyWatchedItemResponse {
            status: "unwatched".to_string(),
            csrfmiddlewaretoken: "TODO".to_string(),
        }))
    }
}

pub async fn update_watched_item(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    http_client: web::Data<Client>,
    query: web::Query<UpdateWatchedItemQueryParams>,
    form_data: web::Json<UpdateWatchedItemFormData>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            let url = query.into_inner().url;
            let res =
                watched_items::get_watched_item(&mut conn_pool, user.id, &normalize_uri(&url))
                    .await?;
            //Handle unwatch
            match form_data.unwatch {
                Some(val) => {
                    if val {
                        return if let Some(item) = res {
                            delete_watched_item(&mut conn_pool, item.user_id, item.document_id)
                                .await?;
                            Ok(HttpResponse::Ok().finish())
                        } else {
                            Err(ApiError::DocumentNotFound)
                        };
                    }
                }
                None => (),
            }
            //Handle create.
            let metadata = get_document_metadata(http_client, &url).await?;
            create_watched_item(&mut conn_pool, user.id, metadata, normalize_uri(&url)).await?;
            Ok(HttpResponse::Ok().finish())
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}

#[derive(Deserialize)]
pub struct UnwatchManyRequest {
    unwatch: Vec<String>,
}

pub async fn unwatch_many(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    urls: web::Json<UnwatchManyRequest>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            let normalized_urls = urls
                .into_inner()
                .unwatch
                .iter()
                .map(|v| normalize_uri(v))
                .collect();
            watched_items::delete_watched_items(&mut conn_pool, user.id, normalized_urls).await?;
        }
        None => return Ok(HttpResponse::Unauthorized().finish()),
    }
    Ok(HttpResponse::Ok().finish())
}
