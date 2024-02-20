use crate::api::root::{RootSetEnforcePlusQuery, RootSetIsAdminQuery};
use crate::db::error::DbError;
use crate::db::model::{User, UserQuery};
use crate::db::schema;
use crate::diesel::ExpressionMethods;
use crate::fxa::FxAUser;
use diesel::{
    insert_into, update, OptionalExtension, PgConnection, QueryDsl, QueryResult, RunQueryDsl,
};

use super::types::Subscription;
use super::v2::multiple_collections::create_default_multiple_collection_for_user;

pub fn root_set_is_admin(
    conn: &mut PgConnection,
    query: RootSetIsAdminQuery,
) -> QueryResult<usize> {
    update(schema::users::table.filter(schema::users::fxa_uid.eq(query.fxa_uid)))
        .set((schema::users::is_admin.eq(query.is_admin),))
        .execute(conn)
}

pub fn root_get_is_admin(conn: &mut PgConnection) -> QueryResult<Vec<String>> {
    schema::users::table
        .filter(schema::users::is_admin.eq(true))
        .select(schema::users::email)
        .get_results(conn)
}

pub fn root_enforce_plus(
    conn: &mut PgConnection,
    query: RootSetEnforcePlusQuery,
) -> QueryResult<usize> {
    update(schema::users::table.filter(schema::users::fxa_uid.eq(query.fxa_uid)))
        .set(schema::users::enforce_plus.eq(query.enforce_plus))
        .execute(conn)
}

pub fn create_or_update_user(
    conn: &mut PgConnection,
    mut fxa_user: FxAUser,
    refresh_token: &str,
) -> Result<usize, diesel::result::Error> {
    if fxa_user.subscriptions.len() > 1 {
        fxa_user.subscriptions.sort();
    }

    let sub: Subscription = fxa_user
        .subscriptions
        .first()
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

    let user_id = insert_into(schema::users::table)
        .values(&user)
        .on_conflict(schema::users::fxa_uid)
        .do_update()
        .set(&user)
        .returning(schema::users::id)
        .get_result(conn)?;

    create_default_multiple_collection_for_user(conn, user_id)
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
