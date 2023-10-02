use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        self,
        error::DbError,
        experiments::{self, create_or_update_experiments},
        model::ExperimentsInsert,
        Pool,
    },
    experiments::{Experiments, ExperimentsConfig},
};

use super::error::ApiError;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ExperimentsUpdate {
    pub active: Option<bool>,
    pub config: Option<ExperimentsConfig>,
}

pub async fn update_experiments(
    _req: HttpRequest,
    user_id: Identity,
    pool: web::Data<Pool>,
    payload: web::Json<ExperimentsUpdate>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = db::users::get_user(&mut conn_pool, user_id.id().unwrap());

    let settings_update = payload.into_inner();
    if let Ok(user) = user {
        if !user.eligible_for_experiments() {
            return Err(ApiError::Forbidden);
        }
        let experiments_insert = ExperimentsInsert {
            user_id: user.id,
            active: settings_update.active,
            config: settings_update.config.unwrap_or_default(),
        };
        let config = create_or_update_experiments(&mut conn_pool, &user, experiments_insert)
            .map_err(DbError::from)?;
        return Ok(HttpResponse::Created().json(config));
    }
    Err(ApiError::InvalidSession)
}

pub async fn get_experiments(
    _req: HttpRequest,
    user_id: Identity,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = db::users::get_user(&mut conn_pool, user_id.id().unwrap());
    if let Ok(user) = user {
        if !user.eligible_for_experiments() {
            return Ok(HttpResponse::Ok().json(None::<Experiments>));
        }
        let exp = experiments::get_experiments(&mut conn_pool, &user)?;
        return Ok(HttpResponse::Ok().json(exp));
    }
    Err(ApiError::InvalidSession)
}
