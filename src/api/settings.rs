use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::db::{
    self,
    error::DbError,
    model::{Settings, SettingsInsert},
    types::Locale,
    Pool,
};

use super::error::ApiError;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SettingUpdateRequest {
    pub locale_override: Option<Option<Locale>>,
    pub mdnplus_newsletter: Option<bool>,
    pub no_ads: Option<bool>,
    pub ai_help_history: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SettingsResponse {
    pub locale_override: Option<Option<Locale>>,
    pub mdnplus_newsletter: Option<bool>,
    pub no_ads: Option<bool>,
    pub ai_help_history: Option<bool>,
}

impl From<Settings> for SettingsResponse {
    fn from(val: Settings) -> Self {
        SettingsResponse {
            locale_override: Some(val.locale_override),
            mdnplus_newsletter: Some(val.mdnplus_newsletter),
            no_ads: Some(val.no_ads),
            ai_help_history: Some(val.ai_help_history),
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
    let user = db::users::get_user(&mut conn_pool, user_id.id()?);

    let settings_update = payload.into_inner();
    if let Ok(user) = user {
        let settings_insert = SettingsInsert {
            user_id: user.id,
            locale_override: settings_update.locale_override,
            mdnplus_newsletter: None,
            no_ads: if user.is_subscriber() {
                settings_update.no_ads
            } else {
                None
            },
            ai_help_history: settings_update.ai_help_history,
        };
        db::settings::create_or_update_settings(&mut conn_pool, settings_insert)
            .map_err(DbError::from)?;
        return Ok(HttpResponse::Created().finish());
    }
    Err(ApiError::InvalidSession)
}
