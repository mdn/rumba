use crate::db::schema::ai_help_history;
use crate::db::Pool;
use crate::diesel::QueryDsl;
use crate::{api::error::ApiError, settings::SETTINGS};
use actix_web::web::Data;
use chrono::Utc;
use diesel::{ExpressionMethods, RunQueryDsl};
use std::ops::Sub;
use std::time::Duration;

/// This removes old AI history records from the database. It is meant to be called from a
/// cron job calling the respective endpoint in the admin API.
pub async fn do_delete_old_ai_history(pool: Data<Pool>) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    let history_deletion_period_in_sec = SETTINGS
        .ai
        .as_ref()
        .map(|ai| ai.history_deletion_period_in_sec)
        .ok_or(ApiError::Generic(
            "ai.history_deletion_period_in_sec missing from configuration".to_string(),
        ))?;

    let oldest_timestamp = Utc::now()
        .sub(Duration::from_secs(history_deletion_period_in_sec))
        .naive_utc();

    let affected_rows = diesel::delete(
        ai_help_history::table.filter(ai_help_history::updated_at.lt(oldest_timestamp)),
    )
    .execute(&mut conn)?;
    info!(
        "Deleted old AI history before {oldest_timestamp}: {affected_rows} old record(s) deleted."
    );
    Ok(())
}
