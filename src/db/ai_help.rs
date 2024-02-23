use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{delete, prelude::*, update};
use diesel::{insert_into, PgConnection};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::error::DbError;
use crate::db::model::{
    AIHelpHistoryInsert, AIHelpHistoryMessage, AIHelpHistoryMessageInsert, AIHelpLimitInsert,
    UserQuery,
};
use crate::db::schema::ai_help_limits as limits;
use crate::db::schema::{ai_help_history, ai_help_history_messages};
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

pub fn add_help_history(
    conn: &mut PgConnection,
    user_id: i64,
    chat_id: Uuid,
) -> Result<(), DbError> {
    let history = AIHelpHistoryInsert {
        user_id,
        chat_id,
        label: String::default(),
        created_at: None,
        updated_at: None,
    };
    insert_into(ai_help_history::table)
        .values(history)
        .on_conflict(ai_help_history::chat_id)
        .do_update()
        .set(ai_help_history::updated_at.eq(diesel::dsl::now))
        .execute(conn)?;
    Ok(())
}

pub fn update_help_history(
    conn: &mut PgConnection,
    user_id: i64,
    chat_id: Uuid,
) -> Result<(), DbError> {
    update(ai_help_history::table)
        .filter(
            ai_help_history::user_id
                .eq(user_id)
                .and(ai_help_history::chat_id.eq(chat_id)),
        )
        .set(ai_help_history::updated_at.eq(diesel::dsl::now))
        .execute(conn)?;
    Ok(())
}

pub fn add_help_history_message(
    conn: &mut PgConnection,
    message: AIHelpHistoryMessageInsert,
) -> Result<NaiveDateTime, DbError> {
    // If a `parent_id` is present, we execute an update on the help history
    // record because one of these are true:
    // * We created a history record at the beginning of the conversation.
    // * History was switched off, we did not create a record and the update
    //   will simply not match/change any record.
    //
    // With no `parent_id`, we create a new record because at this point, history
    // _is_ enabled and we are at the start of a new conversation.
    let res = if message.parent_id.is_some() {
        update_help_history(conn, message.user_id, message.chat_id)
    } else {
        add_help_history(conn, message.user_id, message.chat_id)
    };
    if let Err(err) = res {
        error!("AI Help log: {err}");
    }

    let res = insert_into(ai_help_history_messages::table)
        .values(&message)
        .on_conflict(ai_help_history_messages::message_id)
        .do_update()
        .set(&message)
        .returning(ai_help_history_messages::created_at)
        .get_result::<NaiveDateTime>(conn);
    match res {
        Ok(created_at) => Ok(created_at),
        // Ignore foreign key violations deliberately
        // because of the edge cases described above
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            _info,
        )) => Ok(Utc::now().naive_utc()),
        Err(e) => Err(e.into()),
    }
}

pub fn help_history_get_message(
    conn: &mut PgConnection,
    user: &UserQuery,
    message_id: &Uuid,
) -> Result<Option<AIHelpHistoryMessage>, DbError> {
    ai_help_history_messages::table
        .filter(
            ai_help_history_messages::user_id
                .eq(user.id)
                .and(ai_help_history_messages::message_id.eq(message_id)),
        )
        .first(conn)
        .optional()
        .map_err(Into::into)
}

pub fn help_history(
    conn: &mut PgConnection,
    user: &UserQuery,
    chat_id: &Uuid,
) -> Result<Vec<AIHelpHistoryMessage>, DbError> {
    ai_help_history_messages::table
        .filter(
            ai_help_history_messages::user_id
                .eq(user.id)
                .and(ai_help_history_messages::chat_id.eq(chat_id)),
        )
        .order(ai_help_history_messages::created_at.asc())
        .get_results(conn)
        .map_err(Into::into)
}

#[derive(Queryable, Debug, Default)]
pub struct AIHelpHistoryListEntry {
    pub chat_id: Uuid,
    pub last: NaiveDateTime,
    pub label: String,
}

pub fn list_help_history(
    conn: &mut PgConnection,
    user: &UserQuery,
) -> Result<Vec<AIHelpHistoryListEntry>, DbError> {
    ai_help_history::table
        .filter(ai_help_history::user_id.eq(user.id))
        .select((
            ai_help_history::chat_id,
            ai_help_history::updated_at,
            ai_help_history::label,
        ))
        .order_by((ai_help_history::updated_at.desc(),))
        .get_results(conn)
        .map_err(Into::into)
}

pub fn delete_full_help_history(conn: &mut PgConnection, user: &UserQuery) -> Result<(), DbError> {
    delete(ai_help_history::table.filter(ai_help_history::user_id.eq(user.id))).execute(conn)?;
    Ok(())
}

pub fn delete_help_history(
    conn: &mut PgConnection,
    user: &UserQuery,
    chat_id: Uuid,
) -> Result<bool, DbError> {
    delete(
        ai_help_history_messages::table.filter(
            ai_help_history_messages::chat_id
                .eq(chat_id)
                .and(ai_help_history_messages::user_id.eq(user.id)),
        ),
    )
    .execute(conn)?;
    Ok(delete(
        ai_help_history::table.filter(
            ai_help_history::chat_id
                .eq(chat_id)
                .and(ai_help_history::user_id.eq(user.id)),
        ),
    )
    .execute(conn)?
        == 1)
}

pub fn update_help_history_label(
    conn: &mut PgConnection,
    user: &UserQuery,
    chat_id: Uuid,
    label: &str,
) -> Result<(), DbError> {
    update(ai_help_history::table)
        .filter(
            ai_help_history::user_id
                .eq(user.id)
                .and(ai_help_history::chat_id.eq(chat_id)),
        )
        .set(ai_help_history::label.eq(label))
        .execute(conn)?;
    Ok(())
}
