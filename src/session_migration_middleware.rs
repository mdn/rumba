use crate::{metrics::Metrics, settings::SETTINGS};
use actix_session::SessionExt;
use std::future::{ready, Ready};

use actix_web::{
    cookie::{CookieJar, Key},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error,
};
use futures_util::future::LocalBoxFuture;

pub struct SessionMigration;

impl<S, B> Transform<S, ServiceRequest> for SessionMigration
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
        ready(Ok(SessionMigrationMiddleware { service }))
    }
}

pub struct SessionMigrationMiddleware<S> {
    service: S,
}

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

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if let Some(auth_cookie) = req.cookie(&SETTINGS.auth.auth_cookie_name) {
            let mut jar = CookieJar::new();
            jar.add_original(auth_cookie);
            if let Some(verified_decoded) = jar
                .private(&Key::derive_from(&SETTINGS.auth.auth_cookie_key))
                .get(&SETTINGS.auth.auth_cookie_name)
            {
                let session = req.get_session();
                if let Ok(None) = session.get::<String>("actix_identity.user_id") {
                    let res = session.insert("actix_identity.user_id", verified_decoded.value());
                    if let Some(metrics) = req.app_data::<Data<Metrics>>().cloned() {
                        match res {
                            Ok(_) => metrics.incr("auth_cookie.migration_success"),
                            Err(e) => {
                                error!("error inserting session cookie after migration: {}", e);
                                metrics.incr("auth_cookie.migration_error");
                                sentry::capture_error(&e);
                            }
                        }
                    }
                }
            }
        }

        let fut = self.service.call(req);

        Box::pin(fut)
    }
}
