use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::{insert_into, PgConnection};
use once_cell::sync::Lazy;

use crate::db::error::DbError;
use crate::db::model::{AIHelpLimitInsert, UserQuery};
use crate::db::schema;
use crate::db::schema::ai_help_limits::*;
use crate::settings::SETTINGS;

pub const AI_HELP_LIMIT: i64 = 5;
static AI_HELP_RESET_DURATION: Lazy<Duration> = Lazy::new(|| {
    Duration::seconds(
        SETTINGS
            .ai
            .as_ref()
            .map_or(0, |s| s.limit_reset_duration_in_sec),
    )
});

pub fn get_count(conn: &mut PgConnection, user: &UserQuery) -> Result<i64, DbError> {
    let some_time_ago = Utc::now().naive_utc() - *AI_HELP_RESET_DURATION;
    schema::ai_help_limits::table
        .filter(user_id.eq(&user.id).and(latest_start.gt(some_time_ago)))
        .select(session_questions)
        .first(conn)
        .optional()
        .map(|n| n.unwrap_or(0))
        .map_err(Into::into)
}

pub fn create_or_increment(conn: &mut PgConnection, user: &UserQuery) -> Result<(), DbError> {
    let limit = AIHelpLimitInsert {
        user_id: user.id,
        latest_start: Utc::now().naive_utc(),
        session_questions: 0,
        total_questions: 1,
    };
    insert_into(schema::ai_help_limits::table)
        .values(&limit)
        .on_conflict(schema::ai_help_limits::user_id)
        .do_update()
        .set(((total_questions.eq(total_questions + 1)),))
        .execute(conn)?;
    Ok(())
}

pub fn create_or_increment_limit(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Option<i64>, DbError> {
    let now = Utc::now().naive_utc();
    let limit = AIHelpLimitInsert {
        user_id: user.id,
        latest_start: now,
        session_questions: 1,
        total_questions: 1,
    };
    let some_time_ago = now - *AI_HELP_RESET_DURATION;
    // increment num_question if within limit
    let current = diesel::query_dsl::methods::FilterDsl::filter(
        insert_into(schema::ai_help_limits::table)
            .values(&limit)
            .on_conflict(schema::ai_help_limits::user_id)
            .do_update()
            .set((
                session_questions.eq(session_questions + 1),
                (total_questions.eq(total_questions + 1)),
            )),
        session_questions
            .lt(AI_HELP_LIMIT)
            .and(latest_start.gt(some_time_ago)),
    )
    .returning(session_questions)
    .get_result(conn)
    .optional()?;
    if let Some(current) = current {
        Ok(Some(current))
    } else {
        // reset if latest_start is old enough
        let current = diesel::query_dsl::methods::FilterDsl::filter(
            insert_into(schema::ai_help_limits::table)
                .values(&limit)
                .on_conflict(schema::ai_help_limits::user_id)
                .do_update()
                .set((
                    session_questions.eq(1),
                    (latest_start.eq(now)),
                    (total_questions.eq(total_questions + 1)),
                )),
            latest_start.le(some_time_ago),
        )
        .returning(session_questions)
        .get_result(conn)
        .optional()?;
        Ok(current)
    }
}
