use crate::db::schema;

use diesel::{insert_into, PgConnection};
use diesel::{prelude::*, update};

use crate::db::model::Playground;

pub fn create_playground(conn: &mut PgConnection, playground: Playground) -> QueryResult<usize> {
    insert_into(schema::playground::table)
        .values(&playground)
        .execute(conn)
}

pub fn flag_playground(conn: &mut PgConnection, gist_id: &str) -> QueryResult<usize> {
    update(schema::playground::table.filter(schema::playground::gist.eq(gist_id)))
        .set(schema::playground::flagged.eq(true))
        .execute(conn)
}
