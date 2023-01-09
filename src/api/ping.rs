use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use diesel::{insert_into, ExpressionMethods, PgJsonbExpressionMethods, RunQueryDsl};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::{self, model::ActivityPingInsert, schema::activity_pings, Pool};

use super::error::ApiError;

#[derive(Deserialize)]
pub struct PingQuery {
    pub offline: Option<bool>,
}

pub async fn ping(
    form: web::Form<PingQuery>,
    id: Option<Identity>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    match id {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user = db::users::get_user(&mut conn_pool, id.id().unwrap());
            match user {
                Ok(found) => {
                    let mut activity_data = json!({
                        "subscription_type": found.get_subscription_type()
                    });

                    if form.offline.unwrap_or(false) {
                        // careful: we don't include the offline key
                        // if it's false so the upsert below works.
                        // if we were to include the key, then a false value
                        // from a second client pinging later in the day
                        // could override a true value, which we don't want.
                        activity_data["offline"] = Value::Bool(true);
                    }

                    insert_into(activity_pings::table)
                        .values(ActivityPingInsert {
                            user_id: found.id,
                            activity: activity_data.clone(),
                        })
                        .on_conflict((activity_pings::user_id, activity_pings::ping_at))
                        .do_update()
                        .set(
                            activity_pings::activity
                                .eq(activity_pings::activity.concat(activity_data)),
                        )
                        .execute(&mut conn_pool)?;
                    Ok(HttpResponse::Created().finish())
                }
                Err(_err) => Err(ApiError::InvalidSession),
            }
        }
        None => Err(ApiError::InvalidSession),
    }
}
