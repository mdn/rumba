use chrono::NaiveDateTime;
use diesel::dsl::not;
use diesel::r2d2::ConnectionManager;

use diesel::sql_types::{BigSerial, Bool};
use r2d2::PooledConnection;

use super::error::DbError;
use super::model::{UserQuery, AllNotificationsQuery};
use super::model::{NotificationDataInsert, NotificationInsert, NotificationsQuery};
use crate::api::common::Sorting;
use crate::api::notifications::{NotificationIds, NotificationQueryParams};
use crate::db::schema;
use crate::diesel::BoolExpressionMethods;
use diesel::PgConnection;
use diesel::{insert_into, prelude::*};
use diesel::{update, RunQueryDsl};
use diesel::{QueryDsl, QueryResult};

use crate::diesel::PgTextExpressionMethods;
use diesel::expression_methods::ExpressionMethods;

pub fn get_notifications(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    query: NotificationQueryParams,
) -> Result<Vec<NotificationsQuery>, DbError> {
    let mut notifications_query = schema::notifications::table
        .filter(
            schema::notifications::user_id
                .eq(user_id)
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

    if let Some(unread) = query.unread {
        notifications_query = notifications_query.filter(schema::notifications::read.eq(!unread))
    }
    if let Some(starred) = query.starred {
        notifications_query = notifications_query.filter(schema::notifications::starred.eq(starred))
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

    Ok(notifications_query
        .select((
            schema::notifications::id,
            schema::notification_data::created_at,
            schema::notification_data::updated_at,
            schema::notifications::deleted_at,
            schema::notifications::starred,
            schema::notifications::read,
            schema::notification_data::title,
            schema::notification_data::text,
            schema::documents::uri,
        ))
        .get_results::<NotificationsQuery>(pool)?)
}

pub fn get_all_notifications(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    query: NotificationQueryParams,
) -> Result<Vec<AllNotificationsQuery>, DbError> {
    let mut notifications_query = schema::notification_data::table
        .inner_join(
            schema::documents::table
                .on(schema::documents::id.eq(schema::notification_data::document_id)),
        )
        .into_boxed();

    notifications_query = notifications_query.order_by(schema::notification_data::created_at.desc());

    if let Some(limit) = query.limit {
        notifications_query = notifications_query.limit(limit.into())
    } else {
        notifications_query = notifications_query.limit(10)
    }

    if let Some(offset) = query.offset {
        notifications_query = notifications_query.offset(offset.into())
    }

    if let Some(q) = query.q {
        for browser in q.split(',') {
            notifications_query = notifications_query.or_filter(
                schema::notification_data::data.contains(serde_json::json!({"browsers":[{"browser":browser}]}))
            )
        }
    }

    Ok(notifications_query
        .select((
            schema::notification_data::id,
            schema::notification_data::created_at,
            schema::notification_data::updated_at,
            schema::notification_data::title,
            schema::notification_data::text,
            schema::notification_data::data,
            schema::documents::uri,
        ))
        .get_results::<AllNotificationsQuery>(pool)?)
}

pub fn mark_all_as_read(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user_id))
        .set(schema::notifications::read.eq(true))
        .execute(pool)
}

pub fn set_deleted(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    id: i64,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user_id))
        .filter(schema::notifications::id.eq(id))
        .set(schema::notifications::deleted_at.eq(chrono::offset::Utc::now().naive_utc()))
        .execute(pool)
}

pub fn set_deleted_many(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    ids: Vec<i64>,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user_id))
        .filter(schema::notifications::id.eq_any(ids))
        .set(schema::notifications::deleted_at.eq(chrono::offset::Utc::now().naive_utc()))
        .execute(pool)
}

pub fn clear_deleted(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    id: i64,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user_id))
        .filter(schema::notifications::id.eq(id))
        .set(schema::notifications::deleted_at.eq::<Option<NaiveDateTime>>(None))
        .execute(pool)
}

pub fn mark_as_read(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    id: i64,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user_id))
        .filter(schema::notifications::id.eq(id))
        .set(schema::notifications::read.eq(true))
        .execute(pool)
}

pub fn update_all_starred(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: UserQuery,
    ids: NotificationIds,
    starred: bool,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user.id))
        .filter(schema::notifications::id.eq_any(ids.ids))
        .set(schema::notifications::starred.eq(starred))
        .execute(pool)
}

pub fn toggle_starred(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: UserQuery,
    notification_id: i64,
) -> QueryResult<usize> {
    update(schema::notifications::table)
        .filter(schema::notifications::user_id.eq(user.id))
        .filter(schema::notifications::id.eq(notification_id))
        .set(schema::notifications::starred.eq(not(schema::notifications::starred)))
        .execute(pool)
}

pub fn create_notification(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i64,
    notification_data_id: i64,
) -> QueryResult<i64> {
    let to_create = NotificationInsert {
        deleted_at: None,
        read: false,
        starred: false,
        notification_data_id,
        user_id,
    };

    insert_into(schema::notifications::table)
        .values(&to_create)
        .returning(schema::notifications::id)
        .get_result(pool)
}

pub fn create_notifications_for_users(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    document_id: i64,
    notification_data_id: i64,
) -> QueryResult<i64> {
    let select = schema::watched_items::table
        .select((
            schema::watched_items::user_id,
            false.into_sql::<Bool>(),
            false.into_sql::<Bool>(),
            notification_data_id.into_sql::<BigSerial>(),
        ))
        .filter(schema::watched_items::document_id.eq(document_id));

    let _res = insert_into(schema::notifications::table)
        .values(select)
        .into_columns((
            schema::notifications::user_id,
            schema::notifications::starred,
            schema::notifications::read,
            schema::notifications::notification_data_id,
        ))
        .execute(pool);

    Ok(1)
}

pub fn create_notification_data(
    pool: &mut PooledConnection<ConnectionManager<PgConnection>>,
    notification_data_insert: NotificationDataInsert,
) -> QueryResult<i64> {
    insert_into(schema::notification_data::table)
        .values(&notification_data_insert)
        .returning(schema::notification_data::id)
        .get_result(pool)
}
