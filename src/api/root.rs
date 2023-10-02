use actix_identity::Identity;
use actix_web::{
    dev::HttpServiceFactory,
    web::{self, Data},
    HttpResponse,
};
use serde::Deserialize;

use crate::{
    api::error::ApiError,
    db::{
        model::UserQuery,
        types::Subscription,
        users::{
            find_user_by_email, get_user, root_enforce_plus, root_get_is_admin,
            root_get_is_fox_food, root_get_is_mdn_team, root_set_is_admin, root_set_is_fox_food,
            root_set_is_mdn_team,
        },
        Pool,
    },
};

#[derive(Deserialize)]
pub struct RootQuery {
    email: String,
}

#[derive(Deserialize)]
pub struct RootSetEnforcePlusQuery {
    pub fxa_uid: String,
    pub enforce_plus: Option<Subscription>,
}

#[derive(Deserialize)]
pub struct RootSetIsAdminQuery {
    pub fxa_uid: String,
    pub is_admin: bool,
}

#[derive(Deserialize)]
pub struct RootSetIsFoxFoodQuery {
    pub email: String,
    pub is_fox_food: bool,
}

#[derive(Deserialize)]
pub struct RootSetIsMdnTeamQuery {
    pub email: String,
    pub is_mdn_team: bool,
}

async fn set_enforce_plus(
    pool: Data<Pool>,
    query: web::Json<RootSetEnforcePlusQuery>,
    user_id: Identity,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
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

async fn set_is_mdn_team(
    pool: Data<Pool>,
    query: web::Json<RootSetIsMdnTeamQuery>,
    user_id: Identity,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_set_is_mdn_team(&mut conn_pool, query.into_inner());
    if let Err(e) = res {
        Ok(HttpResponse::BadRequest().json(format!("unable to update user: {}", e)))
    } else {
        Ok(HttpResponse::Created().json("updated"))
    }
}

async fn get_is_mdn_team(pool: Data<Pool>, user_id: Identity) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_get_is_mdn_team(&mut conn_pool)?;
    Ok(HttpResponse::Created().json(res))
}

async fn set_is_fox_food(
    pool: Data<Pool>,
    query: web::Json<RootSetIsFoxFoodQuery>,
    user_id: Identity,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_set_is_fox_food(&mut conn_pool, query.into_inner());
    if let Err(e) = res {
        Ok(HttpResponse::BadRequest().json(format!("unable to update user: {}", e)))
    } else {
        Ok(HttpResponse::Created().json("updated"))
    }
}

async fn get_is_fox_food(pool: Data<Pool>, user_id: Identity) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_get_is_fox_food(&mut conn_pool)?;
    Ok(HttpResponse::Created().json(res))
}

async fn set_is_admin(
    pool: Data<Pool>,
    query: web::Json<RootSetIsAdminQuery>,
    user_id: Identity,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
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

async fn get_is_admin(pool: Data<Pool>, user_id: Identity) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let res = root_get_is_admin(&mut conn_pool)?;
    Ok(HttpResponse::Created().json(res))
}

async fn user_by_email(
    pool: Data<Pool>,
    query: web::Query<RootQuery>,
    user_id: Identity,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if !me.is_admin {
        return Ok(HttpResponse::Forbidden().finish());
    }
    let user = find_user_by_email(&mut conn_pool, query.into_inner().email)?;
    Ok(HttpResponse::Ok().json(user))
}

pub fn root_service() -> impl HttpServiceFactory {
    web::scope("/root")
        .service(web::resource("/").route(web::get().to(user_by_email)))
        .service(
            web::resource("/is-mdn-team")
                .route(web::post().to(set_is_mdn_team))
                .route(web::get().to(get_is_mdn_team)),
        )
        .service(
            web::resource("/is-fox-food")
                .route(web::post().to(set_is_fox_food))
                .route(web::get().to(get_is_fox_food)),
        )
        .service(
            web::resource("/is-admin")
                .route(web::post().to(set_is_admin))
                .route(web::get().to(get_is_admin)),
        )
        .service(web::resource("/enforce-plus").route(web::post().to(set_enforce_plus)))
}
