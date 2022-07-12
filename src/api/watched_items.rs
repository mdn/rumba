use actix_web::{web, HttpRequest, HttpResponse};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2::PooledConnection;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    api::user_middleware::UserId,
    db::{
        self,
        model::{UserQuery, WatchedItemsQuery},
        users::get_user,
        watched_items::{self, create_watched_item, delete_watched_item, get_watched_item_count},
        Pool,
    },
    settings::SETTINGS,
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
    pub subscription_limit_reached: bool,
}

#[derive(Serialize)]
struct SingleWatchedItemResponse {
    #[serde(flatten)]
    pub result: WatchedItem,
    pub csrfmiddlewaretoken: String,
    pub subscription_limit_reached: bool,
}

#[derive(Serialize)]
struct EmptyWatchedItemResponse {
    /* Status will always be 'major' or 'unwatched' for back compat */
    pub status: String,
    pub csrfmiddlewaretoken: String,
    pub subscription_limit_reached: bool,
}

#[derive(Serialize)]
pub struct WatchedItem {
    title: String,
    url: String,
    path: String,
    status: String,
}

#[derive(Serialize)]
pub struct WatchedItemUpdateResponse {
    ok: bool,
    subscription_limit_reached: bool,
    error: Option<String>,
    info: Option<Value>,
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
    user_id: UserId,
    pool: web::Data<Pool>,
    query: web::Query<WatchedItemQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id).await?;
    let query = query.into_inner();
    if let Some(url) = query.url {
        handle_single_item_query(&mut conn_pool, &user, &url).await
    } else {
        handle_paginated_items_query(&mut conn_pool, user, &query).await
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
    let subscription_limit_reached = watched_items_subscription_info_for_user(&user, conn_pool)
        .await?
        .limit_reached;
    Ok(HttpResponse::Ok().json(WatchedItemsResponse {
        items,
        csrfmiddlewaretoken: "TODO".to_string(),
        subscription_limit_reached,
    }))
}

async fn handle_single_item_query(
    conn_pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: &UserQuery,
    url: &str,
) -> Result<HttpResponse, ApiError> {
    let res = watched_items::get_watched_item(conn_pool, user.id, &normalize_uri(url)).await?;

    let subscription_limit_reached = watched_items_subscription_info_for_user(user, conn_pool)
        .await?
        .limit_reached;

    if let Some(item) = res {
        Ok(HttpResponse::Ok().json(SingleWatchedItemResponse {
            result: item.into(),
            csrfmiddlewaretoken: "TODO".to_string(),
            subscription_limit_reached,
        }))
    } else {
        Ok(HttpResponse::Ok().json(EmptyWatchedItemResponse {
            status: "unwatched".to_string(),
            csrfmiddlewaretoken: "TODO".to_string(),
            subscription_limit_reached,
        }))
    }
}

struct WatchedItemsSubscriptionInfo {
    pub limit_reached: bool,
    pub watched_items_remaining: Option<i64>,
}

async fn watched_items_subscription_info_for_user(
    user: &UserQuery,
    conn_pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<WatchedItemsSubscriptionInfo, ApiError> {
    return match user.get_subscription_type() {
        Some(_type) => {
            if matches!(_type, db::types::Subscription::Core) {
                {
                    let count = get_watched_item_count(conn_pool, user.id).await?;
                    let limit_reached =
                        count >= SETTINGS.application.subscriptions_limit_watched_items;

                    let watched_items_remaining =
                        SETTINGS.application.subscriptions_limit_watched_items - count;

                    Ok(WatchedItemsSubscriptionInfo {
                        limit_reached,
                        watched_items_remaining: Some(watched_items_remaining),
                    })
                }
            } else {
                Ok(WatchedItemsSubscriptionInfo {
                    limit_reached: false,
                    watched_items_remaining: None,
                })
            }
        }
        //Strange and impossible 'no subscription' case
        None => Ok(WatchedItemsSubscriptionInfo {
            limit_reached: true,
            watched_items_remaining: Some(0),
        }),
    };
}

pub async fn update_watched_item(
    _req: HttpRequest,
    user_id: UserId,
    pool: web::Data<Pool>,
    http_client: web::Data<Client>,
    query: web::Query<UpdateWatchedItemQueryParams>,
    form_data: web::Json<UpdateWatchedItemFormData>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id).await?;
    let url = query.into_inner().url;
    let res =
        watched_items::get_watched_item(&mut conn_pool, user.id, &normalize_uri(&url)).await?;

    match form_data.unwatch {
        Some(val) => {
            if val {
                let res = handle_unwatch(res, &mut conn_pool, &user).await?;
                return Ok(res);
            }
        }
        None => (),
    }

    let subscription_info = watched_items_subscription_info_for_user(&user, &mut conn_pool).await?;

    if subscription_info.limit_reached {
        return Ok(HttpResponse::BadRequest().json(WatchedItemUpdateResponse {
            ok: false,
            subscription_limit_reached: subscription_info.limit_reached,
            error: Some("max_subscriptions".to_string()),
            info: Some(json!({"max_allowed": SETTINGS
                    .application
                    .subscriptions_limit_watched_items})),
        }));
    }
    //Handle create.
    let metadata = get_document_metadata(http_client, &url).await?;
    let created =
        create_watched_item(&mut conn_pool, user.id, metadata, normalize_uri(&url)).await?;

    let subscription_limit_reached = subscription_info
        .watched_items_remaining
        .map_or(false, |remaining| (remaining - created as i64) <= 0);

    Ok(HttpResponse::Ok().json(WatchedItemUpdateResponse {
        ok: true,
        subscription_limit_reached,
        error: None,
        info: None,
    }))
}

async fn handle_unwatch(
    res: Option<WatchedItemsQuery>,
    conn_pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: &UserQuery,
) -> Result<HttpResponse, ApiError> {
    return if let Some(item) = res {
        delete_watched_item(conn_pool, item.user_id, item.document_id).await?;
        let subscription_limit_reached = watched_items_subscription_info_for_user(user, conn_pool)
            .await?
            .limit_reached;
        Ok(HttpResponse::Ok().json(WatchedItemUpdateResponse {
            ok: true,
            subscription_limit_reached,
            error: None,
            info: None,
        }))
    } else {
        Err(ApiError::DocumentNotFound)
    };
}

#[derive(Deserialize)]
pub struct UnwatchManyRequest {
    unwatch: Vec<String>,
}

pub async fn unwatch_many(
    _req: HttpRequest,
    user_id: UserId,
    pool: web::Data<Pool>,
    urls: web::Json<UnwatchManyRequest>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user: UserQuery = get_user(&mut conn_pool, user_id.id).await?;
    let normalized_urls = urls
        .into_inner()
        .unwatch
        .iter()
        .map(|v| normalize_uri(v))
        .collect();
    watched_items::delete_watched_items(&mut conn_pool, user.id, normalized_urls).await?;
    Ok(HttpResponse::Ok().finish())
}
