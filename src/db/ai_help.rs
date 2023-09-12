use chrono::{Duration, NaiveDateTime, Utc};
use diesel::dsl::sql;
use diesel::sql_types::Text;
use diesel::{insert_into, PgConnection};
use diesel::{prelude::*, update};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::error::DbError;
use crate::db::model::{
    AIHelpLimitInsert, AIHelpLogs, AIHelpLogsFeedbackInsert, AIHelpLogsInsert, UserQuery,
};
use crate::db::schema::ai_help_limits as limits;
use crate::db::schema::ai_help_logs;
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

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackTyp {
    ThumbsDown,
    ThumbsUp,
}

#[derive(Serialize, Deserialize)]
pub struct AIHelpFeedback {
    pub chat_id: Uuid,
    pub message_id: i32,
    pub feedback: Option<String>,
    pub thumbs: Option<FeedbackTyp>,
}

pub fn get_count(conn: &mut PgConnection, user: &UserQuery) -> Result<i64, DbError> {
    let some_time_ago = Utc::now().naive_utc() - *AI_HELP_RESET_DURATION;
    limits::table
        .filter(
            limits::user_id
                .eq(&user.id)
                .and(limits::latest_start.gt(some_time_ago)),
        )
        .select(limits::session_questions)
        .first(conn)
        .optional()
        .map(|n| n.unwrap_or(0))
        .map_err(Into::into)
}

pub fn create_or_increment_total(conn: &mut PgConnection, user: &UserQuery) -> Result<(), DbError> {
    let limit = AIHelpLimitInsert {
        user_id: user.id,
        latest_start: Utc::now().naive_utc(),
        session_questions: 0,
        total_questions: 1,
    };
    insert_into(limits::table)
        .values(&limit)
        .on_conflict(limits::user_id)
        .do_update()
        .set(((limits::total_questions.eq(limits::total_questions + 1)),))
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
        insert_into(limits::table)
            .values(&limit)
            .on_conflict(limits::user_id)
            .do_update()
            .set((
                limits::session_questions.eq(limits::session_questions + 1),
                (limits::total_questions.eq(limits::total_questions + 1)),
            )),
        limits::session_questions
            .lt(AI_HELP_LIMIT)
            .and(limits::latest_start.gt(some_time_ago)),
    )
    .returning(limits::session_questions)
    .get_result(conn)
    .optional()?;
    if let Some(current) = current {
        Ok(Some(current))
    } else {
        // reset if latest_start is old enough
        let current = diesel::query_dsl::methods::FilterDsl::filter(
            insert_into(limits::table)
                .values(&limit)
                .on_conflict(limits::user_id)
                .do_update()
                .set((
                    limits::session_questions.eq(1),
                    (limits::latest_start.eq(now)),
                    (limits::total_questions.eq(limits::total_questions + 1)),
                )),
            limits::latest_start.le(some_time_ago),
        )
        .returning(limits::session_questions)
        .get_result(conn)
        .optional()?;
        Ok(current)
    }
}

pub fn add_help_log(conn: &mut PgConnection, cache: &AIHelpLogsInsert) -> Result<(), DbError> {
    insert_into(ai_help_logs::table)
        .values(cache)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn add_help_log_feedback(
    conn: &mut PgConnection,
    user: &UserQuery,
    chat_id: &Uuid,
    message_id: i32,
    feedback: &AIHelpLogsFeedbackInsert,
) -> Result<(), DbError> {
    update(ai_help_logs::table)
        .filter(
            ai_help_logs::user_id.eq(user.id).and(
                ai_help_logs::chat_id
                    .eq(chat_id)
                    .and(ai_help_logs::message_id.eq(message_id)),
            ),
        )
        .set(feedback)
        .execute(conn)?;
    Ok(())
}

pub fn help_from_log(
    conn: &mut PgConnection,
    user: &UserQuery,
    chat_id: &Uuid,
) -> Result<Vec<AIHelpLogs>, DbError> {
    ai_help_logs::table
        .filter(
            ai_help_logs::user_id
                .eq(user.id)
                .and(ai_help_logs::chat_id.eq(chat_id)),
        )
        .order(ai_help_logs::message_id.asc())
        .get_results(conn)
        .map_err(Into::into)
}

#[derive(Queryable, Deserialize, Serialize, Debug, Default)]
pub struct AIHelpLogsListEntry {
    pub chat_id: Uuid,
    pub last: NaiveDateTime,
    pub question: String,
}

pub fn help_log_list(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Vec<AIHelpLogsListEntry>, DbError> {
    ai_help_logs::table
        .filter(ai_help_logs::user_id.eq(user.id))
        .select((
            ai_help_logs::chat_id,
            ai_help_logs::created_at,
            sql::<Text>("request->'messages'->-1->>'content'"),
        ))
        .order_by((ai_help_logs::chat_id, ai_help_logs::created_at.desc()))
        .distinct_on(ai_help_logs::chat_id)
        .get_results(conn)
        .map_err(Into::into)
}
