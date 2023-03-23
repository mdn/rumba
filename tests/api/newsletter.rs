use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use stubr::{Config, Stubr};

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn whoami_settings_test() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["email"], "test@test.com");

    let newsletter = logged_in_client.get("/api/v1/plus/newsletter/", None).await;

    assert_eq!(newsletter.status(), 200);
    let json = read_json(newsletter).await;
    assert_eq!(json["subscribed"], false);

    let newsletter = logged_in_client
        .post("/api/v1/plus/newsletter/", None, None)
        .await;
    assert_eq!(newsletter.status(), 201);
    let json = read_json(newsletter).await;
    assert_eq!(json["subscribed"], true);

    drop(stubr);
    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/newsletter"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;
    let newsletter = logged_in_client.get("/api/v1/plus/newsletter/", None).await;

    assert_eq!(newsletter.status(), 200);
    let json = read_json(newsletter).await;
    assert_eq!(json["subscribed"], true);

    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["settings"]["mdnplus_newsletter"], true);

    drop(stubr);
    Ok(())
}
