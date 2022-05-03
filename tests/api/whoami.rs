use actix_web::test;
use anyhow::Error;
use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::session::{TestHttpClient};

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn basic() -> Result<(), Error> {
    reset()?;
    let app = test_app_with_login().await.unwrap();
    let app = test::init_service(app).await;
    TestHttpClient::new(app);

    Ok(())
}