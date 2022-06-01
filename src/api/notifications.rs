use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};

use crate::db::{
    model::{NotificationsQuery, UserQuery},
    notifications::{self, get_notifications, update_all_starred},
    users::get_user,
    Pool,
};

use super::{common::Sorting, error::ApiError};

#[derive(Deserialize)]
pub enum NotificationType {
    #[serde(rename = "content")]
    Content,
    #[serde(rename = "compatibility")]
    Compatibility,
}

#[derive(Deserialize)]
pub struct NotificationQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub starred: Option<bool>,
    pub q: Option<String>,
    pub sort: Option<Sorting>,
    pub filter_type: Option<NotificationType>,
}

#[derive(Serialize)]
pub struct Notification {
    pub id: i64,
    pub title: String,
    pub text: String,
    pub url: String,
    pub created: NaiveDateTime,
    pub deleted: bool,
    pub read: bool,
    pub starred: bool,
}

#[derive(Serialize)]
struct NotificationsResponse {
    pub items: Vec<Notification>,
}

#[derive(Deserialize)]
pub struct NotificationIds {
    pub ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct NotificationId {
    pub id: i64,
}

impl From<NotificationsQuery> for Notification {
    fn from(notification: NotificationsQuery) -> Self {
        Notification {
            created: notification.created_at,
            id: notification.id,
            read: notification.read,
            deleted: notification.deleted_at.is_some(),
            starred: notification.starred,
            text: notification.text,
            title: notification.title,
            url: notification.url,
        }
    }
}

pub async fn notifications(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    query: web::Query<NotificationQueryParams>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            let res = get_notifications(&mut conn_pool, user.id, query.0).await?;
            let items = res
                .iter()
                .map(|notification| Into::<Notification>::into(notification.clone()))
                .collect();
            Ok(HttpResponse::Ok().json(NotificationsResponse { items }))
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}

pub async fn mark_all_as_read(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            notifications::mark_all_as_read(&mut conn_pool, user.id).await?;
            Ok(HttpResponse::Ok().finish())
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}

pub async fn delete_by_id(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    notification_id: web::Path<NotificationId>,
) -> Result<HttpResponse, ApiError> {
    if let Some(id) = id.identity() {
        let mut conn_pool = pool.get()?;
        let user: UserQuery = get_user(&mut conn_pool, id).await?;
        let res = notifications::set_deleted(&mut conn_pool, user.id, notification_id.id).await?;
        if res == 0 {
            return Err(ApiError::NotificationNotFound);
        }
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub async fn undo_delete_by_id(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    notification_id: web::Path<NotificationId>,
) -> Result<HttpResponse, ApiError> {
    if let Some(id) = id.identity() {
        let mut conn_pool = pool.get()?;
        let user: UserQuery = get_user(&mut conn_pool, id).await?;
        let res = notifications::clear_deleted(&mut conn_pool, user.id, notification_id.id).await?;
        if res == 0 {
            return Err(ApiError::NotificationNotFound);
        }
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub async fn delete_many(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    data: web::Json<NotificationIds>,
) -> Result<HttpResponse, ApiError> {
    if let Some(id) = id.identity() {
        let mut conn_pool = pool.get()?;
        let user: UserQuery = get_user(&mut conn_pool, id).await?;
        let _res = notifications::set_deleted_many(&mut conn_pool, user.id, data.0.ids).await?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub async fn mark_as_read(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    notification_id: web::Path<NotificationId>,
) -> Result<HttpResponse, ApiError> {
    if let Some(id) = id.identity() {
        let mut conn_pool = pool.get()?;
        let user: UserQuery = get_user(&mut conn_pool, id).await?;
        let res = notifications::mark_as_read(&mut conn_pool, user.id, notification_id.id).await?;
        if res == 0 {
            return Err(ApiError::NotificationNotFound);
        }
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub async fn star_ids(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    data: web::Json<NotificationIds>,
) -> Result<HttpResponse, ApiError> {
    if let Some(id) = id.identity() {
        let mut conn_pool = pool.get()?;
        let user: UserQuery = get_user(&mut conn_pool, id).await?;
        let _res = update_all_starred(&mut conn_pool, user, data.0, true).await?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

pub async fn unstar_ids(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    data: web::Json<NotificationIds>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            let _res = update_all_starred(&mut conn_pool, user, data.0, false).await?;
            Ok(HttpResponse::Ok().finish())
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}

pub async fn toggle_starred(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    notification_id: web::Path<NotificationId>,
) -> Result<HttpResponse, ApiError> {
    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user: UserQuery = get_user(&mut conn_pool, id).await?;
            let res =
                notifications::toggle_starred(&mut conn_pool, user, notification_id.id).await?;
            if res == 0 {
                return Err(ApiError::NotificationNotFound);
            }
            Ok(HttpResponse::Ok().finish())
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}
