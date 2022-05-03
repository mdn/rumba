use actix_identity::Identity;
use actix_session::{Session};
use serde::{Serialize};

use actix_web::{HttpRequest, HttpResponse, web};
use crate::api::error::ApiError;
use crate::db;
use crate::db::Pool;

#[derive(Serialize)]
pub struct GeoInfo {
    country: String,
}

#[derive(Serialize)]
pub struct WhoamiResponse {
    geo: Option<GeoInfo>,
    // #[deprecated(note="Confusing name. We should consider just changing to user_id")]
    username: Option<String>,
    is_authenticated: Option<bool>,
    email: Option<String>,
    avatar_url: Option<String>,
    is_subscriber: Option<bool>,
    subscription_type: Option<String>,
}

const CLOUDFRONT_COUNTRY_HEADER: &str = "CloudFront-Viewer-Country-Name";

pub async fn whoami(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let header_info = _req.headers().get(CLOUDFRONT_COUNTRY_HEADER);

    let country = header_info.map(|header| GeoInfo { country: String::from(header.to_str().unwrap_or("Unknown")) });

    match id.identity() {
        Some(id) => {
            println!("Whoami logged in");
            let user = db::users::get_user(&pool.get().unwrap(), id).await;
            match user {
                Ok(found) => {
                    let response = WhoamiResponse {
                        geo: country,
                        username: Option::Some(found.fxa_uid),
                        subscription_type: Option::Some(found.subscription_type.unwrap().into()),
                        avatar_url: found.avatar_url,
                        is_subscriber: Option::Some(found.is_subscriber),
                        is_authenticated: Option::Some(true),
                        email: Option::Some(found.email),
                    };
                    Ok(HttpResponse::Ok().json(response))
                }
                Err(_err) => Err(ApiError::InvalidSession)
            }
        }
        None => {
            let res = WhoamiResponse { geo: country, username: None, is_authenticated: None, email: None, avatar_url: None, is_subscriber: None, subscription_type: None };
            Ok(HttpResponse::Ok().json(res))
        }
    }
}