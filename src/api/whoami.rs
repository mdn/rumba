use actix_identity::Identity;

use serde::Serialize;

use crate::db;
use crate::db::Pool;
use crate::metrics::Metrics;
use crate::util::country_iso_to_name;
use crate::{api::error::ApiError, db::types::Subscription};
use actix_web::{web, HttpRequest, HttpResponse};

use super::settings::SettingsResponse;

#[derive(Serialize)]
pub struct GeoInfo {
    country: String,
    country_iso: String,
}

#[derive(Serialize, Default)]
pub struct WhoamiResponse {
    geo: Option<GeoInfo>,
    // #[deprecated(note="Confusing name. We should consider just changing to user_id")]
    username: Option<String>,
    is_authenticated: Option<bool>,
    email: Option<String>,
    avatar_url: Option<String>,
    is_subscriber: Option<bool>,
    subscription_type: Option<Subscription>,
    settings: Option<SettingsResponse>,
}

const CLOUDFRONT_COUNTRY_HEADER: &str = "CloudFront-Viewer-Country";
const CLOUDFRONT_COUNTRY_NAME_HEADER: &str = "CloudFront-Viewer-Country-Name";
const GOOGLE_COUNTRY_HEADER: &str = "X-Appengine-Country";

pub async fn whoami(
    req: HttpRequest,
    id: Option<Identity>,
    pool: web::Data<Pool>,
    metrics: Metrics,
) -> Result<HttpResponse, ApiError> {
    let headers = req.headers();

    let country_iso = None
        .or(headers.get(CLOUDFRONT_COUNTRY_HEADER))
        .or(headers.get(GOOGLE_COUNTRY_HEADER))
        .and_then(|header| header.to_str().ok())
        .unwrap_or("ZZ")
        .to_string();

    let country = headers
        .get(CLOUDFRONT_COUNTRY_NAME_HEADER)
        .and_then(|header| header.to_str().ok())
        .or(country_iso_to_name(&country_iso))
        .unwrap_or("Unknown")
        .to_string();

    let geo = GeoInfo {
        country,
        country_iso,
    };

    match id {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user = db::users::get_user(&mut conn_pool, id.id().unwrap());
            match user {
                Ok(user) => {
                    let settings = db::settings::get_settings(&mut conn_pool, &user)?;
                    let subscription_type = user.get_subscription_type().unwrap_or_default();
                    let is_subscriber = user.is_subscriber();
                    let response = WhoamiResponse {
                        geo: Option::Some(geo),
                        username: Option::Some(user.fxa_uid),
                        subscription_type: Option::Some(subscription_type),
                        avatar_url: user.avatar_url,
                        is_subscriber: Some(is_subscriber),
                        is_authenticated: Option::Some(true),
                        email: Option::Some(user.email),
                        settings: settings.map(Into::into),
                    };
                    metrics.incr("whoami.logged_in_success");
                    Ok(HttpResponse::Ok().json(response))
                }
                Err(err) => {
                    metrics.incr("whoami.logged_in_invalid");
                    sentry::capture_error(&err);
                    Err(ApiError::InvalidSession)
                }
            }
        }
        None => {
            metrics.incr("whoami.anonymous");
            let res = WhoamiResponse {
                geo: Option::Some(geo),
                ..Default::default()
            };
            Ok(HttpResponse::Ok().json(res))
        }
    }
}
