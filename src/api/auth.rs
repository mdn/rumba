use actix_http::HttpMessage;
use actix_identity::Identity;
use actix_session::Session;
use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::cookie::{Cookie, CookieJar, Key, SameSite};
use actix_web::{dev::HttpServiceFactory, http, web, Error, HttpRequest, HttpResponse};
use once_cell::sync::Lazy;
use openidconnect::{CsrfToken, Nonce};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::api::error::ApiError;
use crate::db::Pool;
use crate::{
    fxa::{AuthResponse, LoginManager},
    settings::SETTINGS,
};

static BASE_URL: Lazy<Url> = Lazy::new(|| {
    let mut url = SETTINGS.auth.redirect_url.clone();
    url.set_path("");
    url
});

#[derive(Deserialize, Serialize)]
pub struct LoginCookie {
    csrf_token: CsrfToken,
    nonce: Nonce,
    next: Option<String>,
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

fn resolve_redirect(next: &str) -> String {
    match Url::options().base_url(Some(&BASE_URL)).parse(next) {
        Ok(url) if url.origin() == BASE_URL.origin() => url.as_str().to_owned(),
        _ => String::from("/"),
    }
}

impl LoginCookie {
    pub fn removal() -> Cookie<'static> {
        let mut cookie = Cookie::build(&SETTINGS.auth.login_cookie_name, "")
            .http_only(true)
            .path("/")
            .finish();
        cookie.make_removal();
        cookie
    }
}

impl TryFrom<Cookie<'static>> for LoginCookie {
    type Error = ApiError;

    fn try_from(cookie: Cookie<'static>) -> Result<Self, Self::Error> {
        let mut jar = CookieJar::new();
        jar.add_original(cookie);
        match jar
            .private_mut(&Key::derive_from(&SETTINGS.auth.cookie_key))
            .get(&SETTINGS.auth.login_cookie_name)
        {
            Some(cookie) => {
                let login_cookie = serde_json::from_str(cookie.value())?;
                Ok(login_cookie)
            }
            None => Err(ApiError::ServerError),
        }
    }
}

impl TryFrom<LoginCookie> for Cookie<'static> {
    type Error = ApiError;

    fn try_from(login_cookie: LoginCookie) -> Result<Self, Self::Error> {
        let cookie = Cookie::build(
            &SETTINGS.auth.login_cookie_name,
            serde_json::to_string(&login_cookie)?,
        )
        .http_only(true)
        .secure(SETTINGS.auth.auth_cookie_secure)
        .same_site(SameSite::Lax)
        .expires(OffsetDateTime::now_utc() + Duration::minutes(15))
        .path("/")
        .finish();
        let mut jar = CookieJar::new();
        jar.private_mut(&Key::derive_from(&SETTINGS.auth.cookie_key))
            .add(cookie);
        match jar.get(&SETTINGS.auth.login_cookie_name) {
            Some(cookie) => Ok(cookie.to_owned()),
            None => Err(ApiError::ServerError),
        }
    }
}

async fn login_no_prompt(
    query: web::Query<NoPromptQuery>,
    id: Option<Identity>,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id {
        id.logout();
    }
    let NoPromptQuery { next, email } = query.into_inner();
    let (url, csrf_token, nonce) = login_manager.login(email);
    let login_cookie = LoginCookie {
        csrf_token,
        nonce,
        next,
    };
    let cookie = login_cookie.try_into()?;
    Ok(HttpResponse::TemporaryRedirect()
        .cookie(cookie)
        .append_header((http::header::LOCATION, url.as_str()))
        .finish())
}

async fn login(
    query: web::Query<LoginQuery>,
    id: Option<Identity>,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    if let Some(id) = id {
        id.logout();
    }
    let (url, csrf_token, nonce) = login_manager.login(None);
    let login_cookie = LoginCookie {
        csrf_token,
        nonce,
        next: query.into_inner().next,
    };

    let cookie = login_cookie.try_into()?;
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
    let cookie = LoginCookie::removal();
    Ok(HttpResponse::Found()
        .cookie(cookie)
        .append_header((http::header::LOCATION, "/"))
        .finish())
}

async fn callback(
    req: HttpRequest,
    pool: web::Data<Pool>,
    web::Query(q): web::Query<AuthResponse>,
    login_manager: web::Data<LoginManager>,
) -> Result<HttpResponse, Error> {
    if let Some(login_cookie) = req.cookie(&SETTINGS.auth.login_cookie_name) {
        let LoginCookie {
            csrf_token,
            nonce,
            next,
        } = login_cookie.try_into()?;
        if csrf_token.secret() == &q.state {
            let uid = login_manager
                .callback(q.code, nonce, &pool)
                .await
                .map_err(|err| {
                    error!("{:?}", err);
                    actix_web::error::ErrorInternalServerError(err)
                })?;
            Identity::login(&req.extensions(), uid).map_err(|err| {
                error!("{}", err);
                actix_web::error::ErrorInternalServerError(err)
            })?;

            let cookie = LoginCookie::removal();
            let next = match next {
                Some(next) => resolve_redirect(&next),
                _ => String::from("/"),
            };

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
