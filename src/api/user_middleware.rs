use actix_http::HttpMessage;
use actix_identity::RequestIdentity;
use std::future::{ready, Ready};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, FromRequest,
};
use futures_util::future::LocalBoxFuture;

use crate::api::error::ApiError;

pub struct AddUser;

#[derive(Clone)]
pub struct UserId {
    pub id: String,
}

impl FromRequest for UserId {
    type Error = Error;
    type Future = Ready<Result<UserId, Error>>;
    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_http::Payload) -> Self::Future {
        if let Some(user_id) = req.extensions().get::<UserId>() {
            ready(Ok(user_id.clone()))
        } else {
            ready(Err(ApiError::Unauthorized.into()))
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AddUser
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AddUserMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AddUserMiddleware { service }))
    }
}

pub struct AddUserMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AddUserMiddleware<S>
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
        let identity = req.get_identity();
        if let Some(user_id) = identity {
            req.extensions_mut().insert(UserId { id: user_id });
        }

        let fut = self.service.call(req);

        Box::pin(fut)
    }
}
