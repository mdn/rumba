use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{
    api::user_middleware::UserId,
    db::{self, error::DbError, model::Settings, types::Locale, Pool},
};

use super::error::ApiError;

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingUpdateRequest {
    pub col_in_search: Option<bool>,
    pub locale_override: Option<Option<Locale>>,
    pub multiple_collections: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsResponse {
    pub col_in_search: Option<bool>,
    pub locale_override: Option<Option<Locale>>,
    pub multiple_collections: Option<bool>,
    pub collections_last_modified_time: Option<NaiveDateTime>,
}

impl From<Settings> for SettingsResponse {
    fn from(val: Settings) -> Self {
        SettingsResponse {
            col_in_search: Some(val.col_in_search),
            locale_override: Some(val.locale_override),
            multiple_collections: Some(val.multiple_collections),  
            collections_last_modified_time: val.collections_last_modified_time          
        }
    }
}

pub async fn update_settings(
    _req: HttpRequest,
    user_id: UserId,
    pool: web::Data<Pool>,
    payload: web::Json<SettingUpdateRequest>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = db::users::get_user(&mut conn_pool, user_id.id);

    let settings_update = payload.into_inner();
    if let Ok(user) = user {
        db::settings::create_or_update_settings(&mut conn_pool, &user, settings_update)
            .map_err(DbError::from)?;
        return Ok(HttpResponse::Created().finish());
    }
    Err(ApiError::InvalidSession)
}
