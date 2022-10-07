use actix_http::{header::HeaderValue, HttpMessage};
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use actix_web::{
    cookie::{Cookie, CookieJar, Key},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header,
    Error,
};
use futures_util::future::LocalBoxFuture;

pub struct MigrateSessionCookie {
    pub config: Rc<CookieConfig>,
}

impl<S, B> Transform<S, ServiceRequest> for MigrateSessionCookie
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SessionMigrationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SessionMigrationMiddleware {
            service,
            config: Rc::clone(&self.config),
        }))
    }
}

#[derive(Clone)]
pub struct CookieConfig {
    pub cookie_name: String,
    pub cookie_key: Key,
}

pub struct SessionMigrationMiddleware<S> {
    service: S,
    config: Rc<CookieConfig>,
}

#[derive(Debug)]
struct Cookies(Vec<Cookie<'static>>);

impl<S, B> Service<ServiceRequest> for SessionMigrationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let updated_cookies = req
            .cookies()
            .ok()
            .map(|cookies| {
                cookies
                    .iter()
                    .map(|original_cookie| {
                        let mut to_change = original_cookie.clone();
                        if to_change.name() == self.config.cookie_name {
                            let mut jar = CookieJar::new();
                            jar.add_original(to_change.clone());
                            let verified_decoded = jar
                                .private(&self.config.cookie_key)
                                .get(&self.config.cookie_name);

                            if let Some(cookie) = verified_decoded {
                                if !cookie.value().contains("actix_identity.user_id") {
                                    let new_val = format!(
                                        "{{\"actix_identity.user_id\":\"\\\"{}\\\"\"}}",
                                        cookie.value()
                                    );
                                    to_change.set_value(new_val);
                                    //Re-encrypt
                                    jar.private_mut(&self.config.cookie_key).add(to_change);
                                    to_change =
                                        jar.get(&self.config.cookie_name).unwrap().to_owned();
                                }
                            }
                        }
                        to_change
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap();

        let new_cookie_header = updated_cookies
            .iter()
            .map(|cookie| cookie.to_string())
            .collect::<Vec<String>>()
            .join(";");
        //We must first override the Cookie header value in the headers.
        req.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_str(new_cookie_header.as_str()).unwrap(),
        );
        //Then we clear the extensions
        req.extensions_mut().clear();
        //Call req.cookies() to update the internal extension state from the new headers
        let _ = req.cookies();
        let fut = self.service.call(req);
        Box::pin(fut)
    }
}
