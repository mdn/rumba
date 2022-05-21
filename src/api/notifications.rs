use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};

use crate::db::{
    model::{NotificationsQuery, UserQuery},
    notifications::get_notifications,
    users::get_user,
    Pool,
};

use super::{error::ApiError, common::Sorting};


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
    pub read: bool,
    pub starred: bool,
}

#[derive(Serialize)]
struct NotificationsResponse {
    pub items: Vec<Notification>,
}

impl From<NotificationsQuery> for Notification {
    fn from(notification: NotificationsQuery) -> Self {
        Notification {
            created: notification.created_at,
            id: notification.id,
            read: notification.read,
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
            let res = get_notifications(&mut conn_pool, user, query.0).await?;
            let items = res
                .iter()
                .map(|notification| Into::<Notification>::into(notification.clone()))
                .collect();
            Ok(HttpResponse::Ok().json(NotificationsResponse { items }))
        }
        None => Ok(HttpResponse::Unauthorized().finish()),
    }
}
