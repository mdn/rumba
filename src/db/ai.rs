use chrono::{Duration, Utc};
use diesel::{insert_into, PgConnection};
use diesel::{prelude::*, update};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::ai::explain::AI_EXPLAIN_VERSION;
use crate::db::error::DbError;
use crate::db::model::{AIExplainCacheInsert, AIExplainCacheQuery, AIHelpLimitInsert, UserQuery};
use crate::db::schema::ai_explain_cache as explain;
use crate::db::schema::ai_help_limits as limits;
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackTyp {
    ThumbsDown,
    ThumbsUp,
}
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ExplainFeedback {
    pub typ: FeedbackTyp,
    #[serde_as(as = "Base64")]
    pub hash: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
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

pub fn add_explain_answer(
    conn: &mut PgConnection,
    cache: &AIExplainCacheInsert,
) -> Result<(), DbError> {
    insert_into(explain::table)
        .values(cache)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn explain_from_cache(
    conn: &mut PgConnection,
    signature: &Vec<u8>,
    highlighted_hash: &Vec<u8>,
) -> Result<Option<AIExplainCacheQuery>, DbError> {
    let hit = update(explain::table)
        .filter(
            explain::signature
                .eq(signature)
                .and(explain::highlighted_hash.eq(highlighted_hash))
                .and(explain::version.eq(AI_EXPLAIN_VERSION)),
        )
        .set((
            explain::last_used.eq(Utc::now().naive_utc()),
            explain::view_count.eq(explain::view_count + 1),
        ))
        .returning(explain::all_columns)
        .get_result(conn)
        .optional()?;
    Ok(hit)
}

pub fn set_explain_feedback(
    conn: &mut PgConnection,
    feedback: ExplainFeedback,
) -> Result<(), DbError> {
    let ExplainFeedback {
        typ,
        hash,
        signature,
    } = feedback;
    match typ {
        FeedbackTyp::ThumbsDown => update(explain::table)
            .filter(
                explain::signature
                    .eq(signature)
                    .and(explain::highlighted_hash.eq(hash))
                    .and(explain::version.eq(AI_EXPLAIN_VERSION)),
            )
            .set(explain::thumbs_down.eq(explain::thumbs_down + 1))
            .execute(conn)
            .optional()?,
        FeedbackTyp::ThumbsUp => update(explain::table)
            .filter(
                explain::signature
                    .eq(signature)
                    .and(explain::highlighted_hash.eq(hash))
                    .and(explain::version.eq(AI_EXPLAIN_VERSION)),
            )
            .set(explain::thumbs_up.eq(explain::thumbs_up + 1))
            .execute(conn)
            .optional()?,
    };
    Ok(())
}
