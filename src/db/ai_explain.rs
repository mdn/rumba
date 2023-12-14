use chrono::Utc;
use diesel::{insert_into, PgConnection};
use diesel::{prelude::*, update};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::ai::constants::AI_EXPLAIN_VERSION;
use crate::db::ai_help::FeedbackTyp;
use crate::db::error::DbError;
use crate::db::model::{AIExplainCacheInsert, AIExplainCacheQuery};
use crate::db::schema::ai_explain_cache as explain;

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ExplainFeedback {
    pub typ: FeedbackTyp,
    #[serde_as(as = "Base64")]
    pub hash: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
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
