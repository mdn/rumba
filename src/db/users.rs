use crate::api::root::RootUserUpdateQuery;
use crate::db::error::DbError;
use crate::db::model::{User, UserQuery};
use crate::db::schema;
use crate::diesel::ExpressionMethods;
use crate::fxa::FxAUser;
use diesel::{
    insert_into, update, OptionalExtension, PgConnection, QueryDsl, QueryResult, RunQueryDsl,
};

use super::types::Subscription;

pub fn root_update_user(conn: &mut PgConnection, query: RootUserUpdateQuery) -> QueryResult<usize> {
    update(schema::users::table.filter(schema::users::fxa_uid.eq(query.fxa_uid)))
        .set((
            schema::users::is_admin.eq(query.is_admin),
            schema::users::enforce_plus.eq(query.enforce_plus),
        ))
        .execute(conn)
}

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
        avatar_url: Some(fxa_user.avatar),
        email: fxa_user.email,
        subscription_type: sub,
        enforce_plus: None,
        is_admin: None,
    };

    insert_into(schema::users::table)
        .values(&user)
        .on_conflict(schema::users::fxa_uid)
        .do_update()
        .set(&user)
        .execute(conn)
}

pub fn find_user_by_email(
    conn_pool: &mut PgConnection,
    user_email: impl AsRef<str>,
) -> Result<Option<UserQuery>, DbError> {
    schema::users::table
        .filter(schema::users::email.eq(user_email.as_ref()))
        .first::<UserQuery>(conn_pool)
        .optional()
        .map_err(Into::into)
}

pub fn get_user(conn_pool: &mut PgConnection, user: impl AsRef<str>) -> Result<UserQuery, DbError> {
    schema::users::table
        .filter(schema::users::fxa_uid.eq(user.as_ref()))
        .first::<UserQuery>(conn_pool)
        .map_err(Into::into)
}

pub fn get_user_opt(
    conn_pool: &mut PgConnection,
    user: impl AsRef<str>,
) -> Result<Option<UserQuery>, DbError> {
    schema::users::table
        .filter(schema::users::fxa_uid.eq(user.as_ref()))
        .first::<UserQuery>(conn_pool)
        .optional()
        .map_err(Into::into)
}
