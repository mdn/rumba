use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};

use crate::db::{self, error::DbError, model::SettingsQuery, Pool};

use super::error::ApiError;

pub async fn update_settings(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    payload: web::Json<SettingsQuery>,
) -> Result<HttpResponse, ApiError> {
    if let Some(id) = id.identity() {
        let mut conn_pool = pool.get()?;
        let user = db::users::get_user(&mut conn_pool, id).await;

        let settings_update = payload.into_inner();
        if let Ok(user) = user {
            db::settings::create_or_update_settings(&mut conn_pool, &user, settings_update)
                .map_err(DbError::from)?;
            return Ok(HttpResponse::Created().finish());
        }
    }
    Err(ApiError::InvalidSession)
}
