use crate::db::schema;

use diesel::prelude::*;
use diesel::{insert_into, PgConnection};

use crate::api::settings::SettingUpdateRequest;
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
    user: &UserQuery,
    settings_update: SettingUpdateRequest,
) -> QueryResult<usize> {
    let settings = SettingsInsert {
        user_id: user.id,
        col_in_search: settings_update.col_in_search,
        locale_override: settings_update.locale_override,
    };
    insert_into(schema::settings::table)
        .values(&settings)
        .on_conflict(schema::settings::user_id)
        .do_update()
        .set(&settings)
        .execute(conn)
}
