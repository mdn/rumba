use actix_http::body::{BoxBody, EitherBody};
use actix_identity::IdentityService;
use actix_web::{
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    App, Error,
};
use rumba::add_services;

use super::{db::get_pool, identity::TestIdentityPolicy};

pub async fn test_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = Error,
        Config = (),
        InitError = (),
    >,
> {
    let pool = get_pool();
    let app = App::new()
        .wrap(IdentityService::new(TestIdentityPolicy::new()))
        .app_data(pool);
    add_services(app)
}
