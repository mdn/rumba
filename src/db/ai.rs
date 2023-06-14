use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{insert_into, PgConnection};
use once_cell::sync::Lazy;

use crate::db::error::DbError;
use crate::db::model::{AIHelpLimitInsert, UserQuery};
use crate::db::schema;
use crate::db::schema::ai_help_limits::*;
use crate::settings::SETTINGS;

pub const AI_HELP_LIMIT: i64 = 5;
static AI_HELP_RESET_DURATION: Lazy<Duration> =
    Lazy::new(|| Duration::minutes(SETTINGS.chat.as_ref().map_or(0, |s| s.limit_reset_duration)));

fn now_minus_reset_duration() -> NaiveDateTime {
    Utc::now().naive_utc() - *AI_HELP_RESET_DURATION
}

pub fn get_count(conn: &mut PgConnection, user: &UserQuery) -> Result<i64, DbError> {
    let some_time_ago = now_minus_reset_duration();
    schema::ai_help_limits::table
        .filter(user_id.eq(&user.id).and(latest_start.gt(some_time_ago)))
        .select(num_questions)
        .first(conn)
        .optional()
        .map(|n| n.unwrap_or(0))
        .map_err(Into::into)
}
pub fn create_or_increment_limit(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Option<i64>, DbError> {
    let limit = AIHelpLimitInsert {
        user_id: user.id,
        latest_start: Utc::now().naive_utc(),
        num_questions: 1,
    };
    // increment num_question if within limit
    let current = diesel::query_dsl::methods::FilterDsl::filter(
        insert_into(schema::ai_help_limits::table)
            .values(&limit)
            .on_conflict(schema::ai_help_limits::user_id)
            .do_update()
            .set(num_questions.eq(num_questions + 1)),
        num_questions.lt(AI_HELP_LIMIT),
    )
    .returning(num_questions)
    .get_result(conn)
    .optional()?;
    if let Some(current) = current {
        Ok(Some(current))
    } else {
        let some_time_ago = now_minus_reset_duration();
        // reset if latest_start is old enough
        let current = diesel::query_dsl::methods::FilterDsl::filter(
            insert_into(schema::ai_help_limits::table)
                .values(&limit)
                .on_conflict(schema::ai_help_limits::user_id)
                .do_update()
                .set(num_questions.eq(1)),
            latest_start.le(some_time_ago),
        )
        .returning(num_questions)
        .get_result(conn)
        .optional()?;
        Ok(current)
    }
}
