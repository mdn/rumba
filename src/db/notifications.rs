use actix_web::web;
use diesel::r2d2::ConnectionManager;
use r2d2::PooledConnection;

use super::error::DbError;
use super::model::NotificationsQuery;
use super::{model::UserQuery, Pool};
use crate::api::common::Sorting;
use crate::api::notifications::{Notification, NotificationQueryParams};
use crate::db::schema;
use crate::diesel::BoolExpressionMethods;
use diesel::prelude::*;
use diesel::{insert_into, PgConnection};
use diesel::{update, RunQueryDsl};
use diesel::{QueryDsl, QueryResult};

use crate::diesel::PgTextExpressionMethods;
use diesel::expression_methods::ExpressionMethods;

pub async fn get_notifications(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: UserQuery,
    query: NotificationQueryParams,
) -> Result<Vec<NotificationsQuery>, DbError> {
    let mut notifications_query = schema::notifications::table
        .filter(
            schema::notifications::user_id
                .eq(user.id)
                .and(schema::notifications::deleted_at.is_null()),
        )
        .inner_join(schema::notification_data::table)
        .inner_join(
            schema::documents::table
                .on(schema::documents::id.eq(schema::notification_data::document_id)),
        )
        .into_boxed();

    if let Some(query) = &query.q {
        notifications_query = notifications_query
            .filter(schema::notification_data::text.ilike(format!("%{}%", query)))
            .or_filter(schema::notification_data::title.ilike(format!("%{}%", query)))
    }

    notifications_query = match query.sort {
        Some(Sorting::Title) => {
            notifications_query.order_by(schema::notification_data::title.desc())
        }
        Some(Sorting::Created) => {
            notifications_query.order_by(schema::notification_data::created_at.desc())
        }
        None => notifications_query.order_by(schema::notification_data::created_at.desc()),
    };

    if let Some(limit) = query.limit {
        notifications_query = notifications_query.limit(limit.into())
    } else {
        notifications_query = notifications_query.limit(10)
    }

    if let Some(offset) = query.offset {
        notifications_query = notifications_query.offset(offset.into())
    }

    // pub id: u64,
    // pub title: String,
    // pub text: String,
    // pub url: String,
    // pub created: NaiveDateTime,
    // pub read: bool,
    // pub starred: bool,

    Ok(notifications_query
        .select((
            schema::notifications::id,
            schema::notification_data::created_at,
            schema::notification_data::updated_at,
            schema::notifications::starred,
            schema::notifications::read,
            schema::notification_data::title,
            schema::notification_data::text,
            schema::documents::uri,
        ))
        .get_results::<NotificationsQuery>(pool)?)
}
