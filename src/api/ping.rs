use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::{ping::upsert_activity_ping, settings::get_settings, users::get_user, Pool};

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
            let user = get_user(&mut conn_pool, id.id().unwrap());
            match user {
                Ok(found) => {
                    let mut activity_data = json!({
                        "subscription_type": found.get_subscription_type()
                    });
                    let settings = get_settings(&mut conn_pool, &found)?;

                    settings.map(|s| {
                        if s.ai_help_history {
                            activity_data["ai_help_history"] = Value::Bool(true);
                        }
                        if s.no_ads {
                            activity_data["no_ads"] = Value::Bool(true);
                        }
                    });

                    if form.offline.unwrap_or(false) {
                        // careful: we don't include the offline key
                        // if it's false so the upsert below works.
                        // if we were to include the key, then a false value
                        // from a second client pinging later in the day
                        // could override a true value, which we don't want.
                        activity_data["offline"] = Value::Bool(true);
                    }

                    upsert_activity_ping(&mut conn_pool, found, activity_data)?;

                    Ok(HttpResponse::Created().finish())
                }
                Err(_err) => Err(ApiError::InvalidSession),
            }
        }
        None => Err(ApiError::InvalidSession),
    }
}
