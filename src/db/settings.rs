use crate::db::schema;

use diesel::prelude::*;
use diesel::{insert_into, PgConnection};

use crate::db::error::DbError;
use crate::db::model::Settings;
use crate::db::model::SettingsInsert;
use crate::db::model::UserQuery;

pub fn get_settings(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Option<Settings>, DbError> {
    schema::settings::table
        .filter(schema::settings::user_id.eq(user.id))
        .first::<Settings>(conn)
        .optional()
        .map_err(Into::into)
}

pub fn create_or_update_settings(
    conn: &mut PgConnection,
    settings: SettingsInsert,
) -> QueryResult<usize> {
    insert_into(schema::settings::table)
        .values(&settings)
        .on_conflict(schema::settings::user_id)
        .do_update()
        .set(&settings)
        .execute(conn)
}
