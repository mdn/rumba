use actix_identity::Identity;
use actix_session::SessionLength;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::cookie::time::Duration;
use actix_web::cookie::SameSite;
use actix_web::{
    cookie::Key, dev::HttpServiceFactory, http, web, Error, HttpRequest, HttpResponse,
};
use openidconnect::{CsrfToken, Nonce};

use crate::db::Pool;
use crate::{
    fxa::{AuthResponse, LoginManager},
    settings::SETTINGS,
};

async fn login(
    _req: HttpRequest,
    id: Identity,
    session: Session,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    id.forget();
    let (url, csrf_token, nonce) = login_manager.login();
    session.insert("csrf_token", csrf_token)?;
    session.insert("nonce", nonce)?;
    Ok(HttpResponse::TemporaryRedirect()
        .append_header((http::header::LOCATION, url.as_str()))
        .finish())
}

async fn logout(id: Identity, session: Session, _req: HttpRequest) -> Result<HttpResponse, Error> {
    id.forget();
    session.clear();
    Ok(HttpResponse::Found()
        .append_header((http::header::LOCATION, "/"))
        .finish())
}

async fn callback(
    _req: HttpRequest,
    id: Identity,
    pool: web::Data<Pool>,
    session: Session,
    web::Query(q): web::Query<AuthResponse>,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    let csrf_token: Option<CsrfToken> = session.get("csrf_token")?;
    let nonce: Option<Nonce> = session.get("nonce")?;
    session.clear();
    match (csrf_token, nonce) {
        (Some(state), Some(nonce)) if state.secret() == &q.state => {
            println!("callback");
            let uid = login_manager
                .callback(q.code, nonce, &pool)
                .await
                .map_err(|err| {
                    println!("{:?}", err);
                    actix_web::error::ErrorInternalServerError(err)
                })?;
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
        .wrap(
            SessionMiddleware::builder(
                CookieSessionStore::default(),
                Key::from(&SETTINGS.auth.auth_cookie_key),
            )
            .cookie_same_site(SameSite::Lax)
            .cookie_secure(SETTINGS.auth.auth_cookie_secure)
            .session_length(SessionLength::Predetermined {
                max_session_length: Some(Duration::minutes(15)),
            })
            .build(),
        )
        .service(web::resource("/authenticate").route(web::get().to(login)))
        .service(web::resource("/logout").route(web::post().to(logout)))
        .service(web::resource("/callback/").route(web::get().to(callback)))
}
