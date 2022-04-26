use diesel::{insert_into, PgConnection, QueryResult, RunQueryDsl};
use crate::db::schema;
use schema::users::dsl::*;
use crate::db::model::User;
use crate::fxa::{FxAUser};

pub fn create_or_update_user(conn_pool: &PgConnection, mut fxa_user: FxAUser, refresh_token: &String) -> QueryResult<usize> {
    if fxa_user.subscriptions.len() > 1 {
        fxa_user.subscriptions.sort();
    }
    let sub = (*fxa_user.subscriptions.get(0).unwrap()).into();

    let user = User {
        updated_at: chrono::offset::Utc::now().naive_utc(),
        fxa_uid: fxa_user.uid,
        fxa_refresh_token: String::from(refresh_token),
        avatar_url: fxa_user.avatar,
        is_subscriber: !fxa_user.subscriptions.is_empty(),
        subscription_type: sub,
    };

    insert_into(users).values(&user)
        .on_conflict(fxa_uid)
        .do_update()
        .set(&user)
        .execute(conn_pool)
}