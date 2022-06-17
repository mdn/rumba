use crate::db::error::DbError;
use crate::db::model::{User, UserQuery};
use crate::db::schema;
use crate::diesel::ExpressionMethods;
use crate::fxa::FxAUser;
use diesel::{insert_into, OptionalExtension, PgConnection, QueryDsl, QueryResult, RunQueryDsl};
use schema::users::dsl::*;

use super::types::Subscription;

pub fn create_or_update_user(
    conn: &mut PgConnection,
    mut fxa_user: FxAUser,
    refresh_token: &str,
) -> QueryResult<usize> {
    if fxa_user.subscriptions.len() > 1 {
        fxa_user.subscriptions.sort();
    }

    let sub: Subscription = fxa_user
        .subscriptions
        .get(0)
        .cloned()
        .unwrap_or_default()
        .into();

    let user = User {
        updated_at: chrono::offset::Utc::now().naive_utc(),
        fxa_uid: fxa_user.uid,
        fxa_refresh_token: String::from(refresh_token),
        avatar_url: fxa_user.avatar,
        email: fxa_user.email,
        subscription_type: sub,
    };

    insert_into(users)
        .values(&user)
        .on_conflict(fxa_uid)
        .do_update()
        .set(&user)
        .execute(conn)
}

pub async fn get_user(
    conn_pool: &mut PgConnection,
    user: impl AsRef<str>,
) -> Result<UserQuery, DbError> {
    schema::users::table
        .filter(schema::users::fxa_uid.eq(user.as_ref()))
        .first::<UserQuery>(conn_pool)
        .map_err(Into::into)
}

pub async fn get_user_opt(
    conn_pool: &mut PgConnection,
    user: impl AsRef<str>,
) -> Result<Option<UserQuery>, DbError> {
    schema::users::table
        .filter(schema::users::fxa_uid.eq(user.as_ref()))
        .first::<UserQuery>(conn_pool)
        .optional()
        .map_err(Into::into)
}
