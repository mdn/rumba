use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::db::{self, error::DbError, model::Settings, types::Locale, Pool};

use super::error::ApiError;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SettingUpdateRequest {
    pub locale_override: Option<Option<Locale>>,
    pub mdnplus_newsletter: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SettingsResponse {
    pub locale_override: Option<Option<Locale>>,
    pub mdnplus_newsletter: Option<bool>,
}

impl From<Settings> for SettingsResponse {
    fn from(val: Settings) -> Self {
        SettingsResponse {
            locale_override: Some(val.locale_override),
            mdnplus_newsletter: Some(val.mdnplus_newsletter),
        }
    }
}

pub async fn update_settings(
    _req: HttpRequest,
    user_id: Identity,
    pool: web::Data<Pool>,
    payload: web::Json<SettingUpdateRequest>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = db::users::get_user(&mut conn_pool, user_id.id().unwrap());

    let settings_update = payload.into_inner();
    if let Ok(user) = user {
        db::settings::create_or_update_settings(&mut conn_pool, &user, settings_update)
            .map_err(DbError::from)?;
        return Ok(HttpResponse::Created().finish());
    }
    Err(ApiError::InvalidSession)
}
