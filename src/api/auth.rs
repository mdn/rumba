use std::sync::{Arc, RwLock};

use actix_identity::Identity;
use actix_web::{dev::HttpServiceFactory, http, web, Error, HttpRequest, HttpResponse};

use crate::fxa::{AuthResponse, LoginManager};

async fn login(
    _req: HttpRequest,
    id: Identity,
    login_manager: web::Data<Arc<RwLock<LoginManager>>>,
) -> Result<HttpResponse, Error> {
    let (url, csrf_token) = login_manager
        .try_write()
        .map_err(|_| actix_web::error::ErrorInternalServerError("login"))?
        .login();
    id.remember(csrf_token.secret().to_owned());
    Ok(HttpResponse::TemporaryRedirect()
        .append_header((http::header::LOCATION, url.as_str()))
        .finish())
}

async fn logout(id: Identity, _req: HttpRequest) -> Result<HttpResponse, Error> {
    id.forget();
    Ok(HttpResponse::Found()
        .append_header((http::header::LOCATION, "/"))
        .finish())
}

async fn callback(
    _req: HttpRequest,
    id: Identity,
    web::Query(q): web::Query<AuthResponse>,
    login_manager: web::Data<Arc<RwLock<LoginManager>>>,
) -> Result<HttpResponse, Error> {
    match id.identity() {
        Some(state) if state == q.state => {
            println!("callback");
            let mut lm = login_manager
                .try_write()
                .map_err(|_| actix_web::error::ErrorInternalServerError("lock"))?;
            let uid = lm
                .callback(q.code)
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?;
            id.remember(uid);

            return Ok(HttpResponse::TemporaryRedirect()
                .append_header((http::header::LOCATION, "/"))
                .finish());
        }
        _ => Ok(HttpResponse::Unauthorized().finish()),
    }
}

pub fn auth_service() -> impl HttpServiceFactory {
    web::scope("/users/fxa/login")
        .service(web::resource("/authenticate").route(web::get().to(login)))
        .service(web::resource("/logout").route(web::post().to(logout)))
        .service(web::resource("/callback/").route(web::get().to(callback)))
}
