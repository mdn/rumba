use std::future::{ready, Ready};

use actix_identity::IdentityPolicy;
use actix_web::Error;

pub struct TestIdentityPolicy {}

impl TestIdentityPolicy {
    pub fn new() -> Self {
        Self {}
    }
}

impl IdentityPolicy for TestIdentityPolicy {
    type Future = Ready<Result<Option<String>, Error>>;
    type ResponseFuture = Ready<Result<(), Error>>;

    fn from_request(&self, req: &mut actix_web::dev::ServiceRequest) -> Self::Future {
        ready(Ok(req
            .cookie("test-auth")
            .map(|cookie| cookie.value().to_owned())))
    }

    fn to_response<B>(
        &self,
        _identity: Option<String>,
        _changed: bool,
        _response: &mut actix_web::dev::ServiceResponse<B>,
    ) -> Self::ResponseFuture {
        ready(Ok(()))
    }
}
