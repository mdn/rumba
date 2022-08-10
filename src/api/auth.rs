use actix_http::HttpMessage;
use actix_identity::Identity;
use actix_session::Session;
use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{dev::HttpServiceFactory, http, web, Error, HttpRequest, HttpResponse};
use openidconnect::{CsrfToken, Nonce};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::db::Pool;
use crate::{
    fxa::{AuthResponse, LoginManager},
    settings::SETTINGS,
};

#[derive(Deserialize, Serialize)]
pub struct LoginCookie {
    csrf_token: CsrfToken,
    nonce: Nonce,
}

#[derive(Deserialize)]
pub struct LoginQuery {
    next: Option<String>,
}

#[derive(Deserialize)]
pub struct NoPromptQuery {
    next: Option<String>,
    email: Option<String>,
}

fn build_login_cookie(login_cookie: &LoginCookie) -> Result<Cookie<'static>, ApiError> {
    Ok(Cookie::build(
        &SETTINGS.auth.login_cookie_name,
        serde_json::to_string(login_cookie)?,
    )
    .http_only(true)
    .secure(SETTINGS.auth.auth_cookie_secure)
    .same_site(SameSite::Lax)
    .expires(OffsetDateTime::now_utc() + Duration::minutes(15))
    .path("/")
    .finish())
}
fn build_login_cookie_removal() -> Cookie<'static> {
    let mut cookie = Cookie::build(&SETTINGS.auth.login_cookie_name, "")
        .http_only(true)
        .path("/")
        .finish();
    cookie.make_removal();
    cookie
}

async fn login_no_prompt(
    query: web::Query<NoPromptQuery>,
    id: Option<Identity>,
    session: Session,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id {
        id.logout();
    }
    let NoPromptQuery { next, email } = query.into_inner();
    let (url, csrf_token, nonce) = login_manager.login(email);
    let login_cookie = LoginCookie { csrf_token, nonce };
    if let Some(next) = next {
        session.insert("next", next)?;
    }
    let cookie = build_login_cookie(&login_cookie)?;
    Ok(HttpResponse::TemporaryRedirect()
        .cookie(cookie)
        .append_header((http::header::LOCATION, url.as_str()))
        .finish())
}

async fn login(
    query: web::Query<LoginQuery>,
    id: Option<Identity>,
    session: Session,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id {
        id.logout();
    }
    let (url, csrf_token, nonce) = login_manager.login(None);
    let login_cookie = LoginCookie { csrf_token, nonce };
    if let Some(next) = query.into_inner().next {
        session.insert("next", next)?;
    }

    let cookie = build_login_cookie(&login_cookie)?;
    Ok(HttpResponse::TemporaryRedirect()
        .cookie(cookie)
        .append_header((http::header::LOCATION, url.as_str()))
        .finish())
}

async fn logout(
    id: Option<Identity>,
    session: Session,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id {
        id.logout();
    }
    session.clear();
    let cookie = build_login_cookie_removal();
    Ok(HttpResponse::Found()
        .cookie(cookie)
        .append_header((http::header::LOCATION, "/"))
        .finish())
}

async fn callback(
    req: HttpRequest,
    pool: web::Data<Pool>,
    session: Session,
    web::Query(q): web::Query<AuthResponse>,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    if let Some(login_cookie) = req.cookie(&SETTINGS.auth.login_cookie_name) {
        let LoginCookie { csrf_token, nonce } = serde_json::from_str(login_cookie.value())?;
        if csrf_token.secret() == &q.state {
            debug!("callback");
            let next: String = session.get("next")?.unwrap_or_else(|| String::from("/"));
            let uid = login_manager
                .callback(q.code, nonce, &pool)
                .await
                .map_err(|err| {
                    println!("{:?}", err);
                    actix_web::error::ErrorInternalServerError(err)
                })?;
            Identity::login(&req.extensions(), uid).map_err(|err| {
                error!("{}", err);
                actix_web::error::ErrorInternalServerError(err)
            })?;

            let cookie = build_login_cookie_removal();

            return Ok(HttpResponse::TemporaryRedirect()
                .cookie(cookie)
                .append_header((http::header::LOCATION, next))
                .finish());
        }
    }
    Ok(HttpResponse::Unauthorized().finish())
}

pub fn auth_service() -> impl HttpServiceFactory {
    web::scope("/users/fxa/login")
        .service(web::resource("/no-prompt/").route(web::get().to(login_no_prompt)))
        .service(web::resource("/authenticate/").route(web::get().to(login)))
        .service(web::resource("/logout/").route(web::post().to(logout)))
        .service(web::resource("/callback/").route(web::get().to(callback)))
}
