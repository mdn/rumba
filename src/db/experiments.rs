use diesel::insert_into;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::db::error::DbError;
use crate::db::model::ExperimentsInsert;
use crate::db::model::ExperimentsQuery;
use crate::db::model::UserQuery;
use crate::db::schema::experiments as ex;
use crate::experiments::Experiments;

pub fn get_experiments(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Option<Experiments>, DbError> {
    if !(user.is_admin || user.is_fox_food || user.is_mdn_team) {
        return Ok(None);
    }
    ex::table
        .filter(ex::user_id.eq(user.id).and(ex::active.eq(true)))
        .first::<ExperimentsQuery>(conn)
        .optional()
        .map_err(Into::into)
        .map(|x| x.map(Experiments::from).unwrap_or_default().restrict(user))
}

pub fn create_or_update_experiments(
    conn: &mut PgConnection,
    user: &UserQuery,
    experiments: ExperimentsInsert,
) -> QueryResult<Option<Experiments>> {
    if let Some(res) = match &experiments.active {
        Some(false) => insert_into(ex::table)
            .values(&experiments)
            .on_conflict(ex::user_id)
            .do_update()
            .set((
                ex::active.eq(false),
                ex::config.eq(ex::config.concat(&experiments.config)),
            ))
            .returning(ex::all_columns)
            .get_result::<ExperimentsQuery>(conn)
            .optional()?,
        Some(true) => insert_into(ex::table)
            .values(&experiments)
            .on_conflict(ex::user_id)
            .do_update()
            .set((
                ex::active.eq(true),
                ex::config.eq(ex::config.concat(&experiments.config)),
            ))
            .returning(ex::all_columns)
            .get_result::<ExperimentsQuery>(conn)
            .optional()?,
        None => insert_into(ex::table)
            .values(&experiments)
            .on_conflict(ex::user_id)
            .do_update()
            .set(ex::config.eq(ex::config.concat(&experiments.config)))
            .returning(ex::all_columns)
            .get_result::<ExperimentsQuery>(conn)
            .optional()?,
    } {
        return Ok(if res.active {
            Experiments::from(res).restrict(user)
        } else {
            Experiments::default().restrict(user)
        });
    }
    Ok(None)
}
