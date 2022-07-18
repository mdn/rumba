use actix_web::{
    dev::HttpServiceFactory,
    web::{self, Data},
    HttpResponse,
};
use serde::Deserialize;

use crate::{
    api::{error::ApiError, user_middleware::UserId},
    db::{
        model::UserQuery,
        types::Subscription,
        users::{find_user_by_email, get_user, root_enforce_plus, root_set_is_admin},
        Pool,
    },
};

#[derive(Deserialize)]
pub struct RootQuery {
    email: String,
}

#[derive(Deserialize)]
pub struct RootEnforcePlusQuery {
    pub fxa_uid: String,
    pub enforce_plus: Option<Subscription>,
}

#[derive(Deserialize)]
pub struct RootIsAdminQuery {
    pub fxa_uid: String,
    pub is_admin: bool,
}

async fn set_enforce_plus(
    pool: Data<Pool>,
    query: web::Json<RootEnforcePlusQuery>,
    user_id: UserId,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_enforce_plus(&mut conn_pool, query.into_inner());
    if let Err(e) = res {
        Ok(HttpResponse::BadRequest().json(format!("unable to update user: {}", e)))
    } else {
        Ok(HttpResponse::Created().json("updated"))
    }
}

async fn set_is_admin(
    pool: Data<Pool>,
    query: web::Json<RootIsAdminQuery>,
    user_id: UserId,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_set_is_admin(&mut conn_pool, query.into_inner());
    if let Err(e) = res {
        Ok(HttpResponse::BadRequest().json(format!("unable to update user: {}", e)))
    } else {
        Ok(HttpResponse::Created().json("updated"))
    }
}

async fn user_by_email(
    pool: Data<Pool>,
    query: web::Query<RootQuery>,
    user_id: UserId,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let user = find_user_by_email(&mut conn_pool, query.into_inner().email)?;
    Ok(HttpResponse::Ok().json(user))
}

pub fn root_service() -> impl HttpServiceFactory {
    web::scope("/root")
        .service(web::resource("/").route(web::get().to(user_by_email)))
        .service(web::resource("/is-admin").route(web::post().to(set_is_admin)))
        .service(web::resource("/enforce-plus").route(web::post().to(set_enforce_plus)))
}
