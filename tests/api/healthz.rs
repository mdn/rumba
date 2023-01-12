use actix_web::test;
use anyhow::Error;

use crate::helpers::{app::test_app, db::reset};

#[actix_rt::test]
async fn basic() -> Result<(), Error> {
    let pool = reset()?;
    let app = test_app(&pool).await;
    let app = test::init_service(app).await;
    let req = test::TestRequest::get().uri("/healthz").to_request();
    let res = test::call_service(&app, req).await;
    assert!(res.status().is_success());
    Ok(())
}
