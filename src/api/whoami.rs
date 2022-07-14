use actix_identity::Identity;

use serde::Serialize;

use crate::api::settings::SettingUpdateRequest;
use crate::db;
use crate::db::Pool;
use crate::metrics::Metrics;
use crate::{api::error::ApiError, db::types::Subscription};
use actix_web::{web, HttpRequest, HttpResponse};

#[derive(Serialize)]
pub struct GeoInfo {
    country: String,
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
    settings: Option<SettingUpdateRequest>,
}

const CLOUDFRONT_COUNTRY_HEADER: &str = "CloudFront-Viewer-Country-Name";

pub async fn whoami(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    metrics: Metrics,
) -> Result<HttpResponse, ApiError> {
    let header_info = _req.headers().get(CLOUDFRONT_COUNTRY_HEADER);

    let country = header_info.map(|header| GeoInfo {
        country: String::from(header.to_str().unwrap_or("Unknown")),
    });

    match id.identity() {
        Some(id) => {
            let mut conn_pool = pool.get()?;
            let user = db::users::get_user(&mut conn_pool, id).await;
            match user {
                Ok(found) => {
                    let settings = db::settings::get_settings(&mut conn_pool, &found)?;
                    let subscription_type = found.get_subscription_type().unwrap_or_default();
                    let is_subscriber = subscription_type.is_subscriber();
                    let response = WhoamiResponse {
                        geo: country,
                        username: Option::Some(found.fxa_uid),
                        subscription_type: Option::Some(subscription_type),
                        avatar_url: found.avatar_url,
                        is_subscriber: Some(is_subscriber),
                        is_authenticated: Option::Some(true),
                        email: Option::Some(found.email),
                        settings: settings.map(Into::into),
                    };
                    metrics.incr("whoami.logged_in_success");
                    Ok(HttpResponse::Ok().json(response))
                }
                Err(_err) => {
                    metrics.incr("whoami.logged_in_invalid");
                    Err(ApiError::InvalidSession)
                }
            }
        }
        None => {
            metrics.incr("whoami.anonymous");
            let res = WhoamiResponse {
                geo: country,
                ..Default::default()
            };
            Ok(HttpResponse::Ok().json(res))
        }
    }
}
