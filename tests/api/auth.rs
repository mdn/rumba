use std::collections::HashMap;

use actix_web::test;
use anyhow::Error;
use url::Url;

use crate::helpers::{app::test_app_with_login, db::reset, http_client::check_stubr_initialized};

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_next() -> Result<(), Error> {
    let pool = reset()?;
    let _stubr_ok = check_stubr_initialized().await;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;

    let login_req = test::TestRequest::get()
        .uri("/users/fxa/login/authenticate/?next=/foo")
        .to_request();

    let login_res = test::call_service(&service, login_req).await;

    let location_header = login_res
        .response()
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let cookies = login_res.response().cookies();

    let params: HashMap<_, _> = Url::parse(location_header)
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect();
    let state = params.get("state").to_owned().unwrap().clone();
    let mut base = test::TestRequest::get().uri(&format!(
        "/users/fxa/login/callback/?code={:1}&state={:2}",
        "ABC123", state
    ));
    for cookie in cookies {
        base = base.cookie(cookie);
    }

    let res = test::call_service(&service, base.to_request()).await;

    assert_eq!(
        res.response()
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok()),
        Some("http://localhost:8000/foo")
    );
    Ok(())
}
