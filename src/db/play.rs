use crate::db::model::PlaygroundInsert;
use crate::db::schema::playground;

use diesel::dsl::exists;
use diesel::{insert_into, select, PgConnection};
use diesel::{prelude::*, update};

use super::model::UserQuery;

pub fn create_playground(
    conn: &mut PgConnection,
    playground: PlaygroundInsert,
) -> QueryResult<usize> {
    insert_into(playground::table)
        .values(&playground)
        .execute(conn)
}

pub fn is_playground_created_by_user(
    conn: &mut PgConnection,
    gist_id: &str,
    user: UserQuery,
) -> QueryResult<bool> {
    select(exists(
        playground::table.filter(
            playground::gist
                .eq(gist_id)
                .and(playground::user_id.eq(user.id)),
        ),
    ))
    .get_result(conn)
}

pub fn flag_playground(conn: &mut PgConnection, gist_id: &str) -> QueryResult<usize> {
    update(playground::table.filter(playground::gist.eq(gist_id)))
        .set(playground::flagged.eq(true))
        .execute(conn)
}
