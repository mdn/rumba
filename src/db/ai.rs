use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::query_dsl::methods::FilterDsl;
use diesel::{insert_into, PgConnection};
use once_cell::sync::Lazy;

use crate::db::error::DbError;
use crate::db::model::{AIHelpLimitInsert, UserQuery};
use crate::db::schema;
use crate::db::schema::ai_help_limits::*;
use crate::settings::SETTINGS;

const AI_HELP_LIMIT: i64 = 5;
static AI_HELP_RESET_DURATION: Lazy<Duration> =
    Lazy::new(|| Duration::minutes(SETTINGS.chat.as_ref().map_or(0, |s| s.limit_reset_duration)));

pub fn create_or_increment_limit(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Option<i64>, DbError> {
    let limit = AIHelpLimitInsert {
        user_id: user.id,
        latest_start: chrono::offset::Utc::now().naive_utc(),
        num_questions: 1,
    };
    // increment num_question if within limit
    let current = insert_into(schema::ai_help_limits::table)
        .values(&limit)
        .on_conflict(schema::ai_help_limits::user_id)
        .do_update()
        .set(num_questions.eq(num_questions + 1))
        .filter(num_questions.le(AI_HELP_LIMIT))
        .returning(num_questions)
        .get_result(conn)
        .optional()?;
    if let Some(current) = current {
        Ok(Some(current))
    } else {
        let some_time_ago = Utc::now().naive_utc() - *AI_HELP_RESET_DURATION;
        // reset if latest_start is old enough
        let current = insert_into(schema::ai_help_limits::table)
            .values(&limit)
            .on_conflict(schema::ai_help_limits::user_id)
            .do_update()
            .set(num_questions.eq(1))
            .filter(latest_start.le(some_time_ago))
            .returning(num_questions)
            .get_result(conn)
            .optional()?;
        Ok(current)
    }
}
