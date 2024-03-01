use crate::api::error::ApiError;
use crate::db::Pool;
use actix_rt::ArbiterHandle;
use actix_web::{web::Data, HttpResponse};

pub async fn delete_old_ai_history(
    pool: Data<Pool>,
    arbiter: Data<ArbiterHandle>,
) -> Result<HttpResponse, ApiError> {
    if !arbiter.spawn(async move {
        if let Err(e) = do_delete_old_ai_history(pool).await {
            error!("{}", e);
        }
    }) {
        return Ok(HttpResponse::InternalServerError().finish());
    }
    Ok(HttpResponse::Accepted().finish())
}

async fn do_delete_old_ai_history(pool: Data<Pool>) -> Result<(), ApiError> {
    let mut _conn = pool.get()?;
    info!("Deleting old history");
    Ok(())
}
