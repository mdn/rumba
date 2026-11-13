use crate::helpers::app::{init_test, start_stubr, test_app_with_login};
use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::read_json;
use actix_web::test;
use anyhow::Error;
use serde_json::json;

#[actix_rt::test]
async fn settings_newsletter_subscribe() -> Result<(), Error> {
    let stubr = start_stubr(vec!["tests/stubs"]);
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    assert_eq!(json["geo"]["country_iso"], "IS");

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
    Ok(())
}

#[actix_rt::test]
async fn settings_newsletter_test() -> Result<(), Error> {
    let stubr = start_stubr(vec![
        "tests/stubs",
        "tests/test_specific_stubs/newsletter/basket_lookup_user.json",
    ]);
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let newsletter = logged_in_client.get("/api/v1/plus/newsletter/", None).await;

    assert_eq!(newsletter.status(), 200);
    let json = read_json(newsletter).await;
    assert_eq!(json["subscribed"], true);

    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["settings"]["mdnplus_newsletter"], true);
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn anonymous_newsletter_test() -> Result<(), Error> {
    let stubr = start_stubr(vec!["tests/stubs"]);
    let pool = reset()?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let request = test::TestRequest::post()
        .set_json(json!({ "email": "foo@bar.com"}))
        .uri("/api/v1/newsletter")
        .to_request();
    let newsletter_res = test::call_service(&service, request).await;

    assert!(newsletter_res.status().is_success());
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn anonymous_newsletter_error_test() -> Result<(), Error> {
    let (_, stubr) = init_test(vec![
        "tests/stubs",
        "tests/test_specific_stubs/newsletter/basket_subscribe_error.json",
    ])
    .await?;

    let pool = reset()?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let request = test::TestRequest::post()
        .set_json(json!({ "email": "foo@bar.com"}))
        .uri("/api/v1/newsletter")
        .to_request();
    let newsletter_res = test::call_service(&service, request).await;

    assert!(newsletter_res.status().is_server_error());
    drop(stubr);
    Ok(())
}
