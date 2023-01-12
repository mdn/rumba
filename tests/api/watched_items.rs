use std::thread;
use std::time::Duration;

use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::{read_json, wait_for_stubr, RumbaTestResponse};
use actix_web::dev::Service;
use actix_web::test;
use anyhow::Error;
use serde_json::json;
use stubr::{Config, Stubr};

#[actix_rt::test]
async fn test_create_get_watched_items() -> Result<(), Error> {
    let pool = reset()?;

    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let mut offset = 0;

    create_watched_items(&mut logged_in_client).await;

    let mut res = logged_in_client
        .get(
            format!("/api/v1/plus/watching/?offset={}&limit=10", offset).as_str(),
            None,
        )
        .await;

    assert_eq!(res.response().status(), 200);
    let res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 10);

    // Test properties + sort order is defaulting to most recent first

    assert_eq!(
        res_json["items"][0]["title"],
        "CSS: Cascading Style Sheets 11"
    );
    //API returns lower-cased url.
    assert_eq!(res_json["items"][0]["url"], "/en-us/docs/web/css11");
    assert_eq!(res_json["items"][0]["path"], "docs.web.css.11");
    assert_eq!(res_json["items"][0]["status"], "major");

    // Test CSS 1.json multiple BCD tables only takes 'first' in order.
    offset = 9;
    res = logged_in_client
        .get(
            format!("/api/v1/plus/watching/?offset={}", offset).as_str(),
            None,
        )
        .await;
    assert_eq!(res.response().status(), 200);
    let res_json = read_json(res).await;

    assert_eq!(res_json["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        res_json["items"][0]["title"],
        "B CSS: Cascading Style Sheets"
    );
    assert_eq!(res_json["items"][0]["url"], "/en-us/docs/web/css2");
    //No BCD table - should fall back to url.
    assert_eq!(res_json["items"][0]["path"], "/en-us/docs/web/css2");

    assert_eq!(
        res_json["items"][1]["title"],
        "A CSS: Cascading Style Sheets"
    );
    assert_eq!(res_json["items"][1]["url"], "/en-us/docs/web/css1");
    assert_eq!(
        res_json["items"][1]["path"],
        "docs.web.css.1.first.bcd.in.array"
    );

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_unwatch_many() -> Result<(), Error> {
    let pool = reset()?;

    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    create_watched_items(&mut logged_in_client).await;
    let res = logged_in_client
        .get("/api/v1/plus/watching/?limit=20", None)
        .await;

    let res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 11);

    let to_unwatch = vec![
        res_json["items"][1]["url"].as_str().unwrap(),
        res_json["items"][2]["url"].as_str().unwrap(),
        res_json["items"][3]["url"].as_str().unwrap(),
    ];

    let res = logged_in_client
        .post(
            "/api/v1/plus/unwatch-many/",
            None,
            Some(PostPayload::Json(json!({ "unwatch": to_unwatch }))),
        )
        .await;
    assert_eq!(res.response().status(), 200);
    let res_json = read_json(res).await;
    assert_eq!(res_json["ok"], true);

    let res = logged_in_client
        .get("/api/v1/plus/watching/?limit=20", None)
        .await;

    let res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 8);
    let vals: Vec<&str> = res_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["url"].as_str().unwrap())
        .collect();

    to_unwatch.iter().for_each(|v| assert!(!vals.contains(v)));
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_single_item_operations() -> Result<(), Error> {
    let pool = reset()?;
    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let base_url = "/api/v1/plus/watching/?url=/en-US/docs/Web/CSS";

    let result = logged_in_client.get(base_url, None).await;

    assert_eq!(result.status(), 200);
    let mut res_json = read_json(result).await;

    assert_eq!(res_json["status"].as_str().unwrap(), "unwatched");

    logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "title": "CSS: Cascading Style Sheets",
                "path": "this.gets.ignored",
            }))),
        )
        .await;

    let result = logged_in_client.get(base_url, None).await;

    res_json = read_json(result).await;
    assert_eq!(res_json["status"].as_str().unwrap(), "major");
    assert_eq!(
        res_json["title"].as_str().unwrap(),
        "CSS: Cascading Style Sheets"
    );
    assert_eq!(res_json["url"].as_str().unwrap(), "/en-us/docs/web/css");
    assert_eq!(
        res_json["path"].as_str().unwrap(),
        "css.properties.background-blend-mode"
    );

    let res = logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "unwatch": true,
            }))),
        )
        .await;
    assert_eq!(res.status(), 200);

    let result = logged_in_client.get(base_url, None).await;

    res_json = read_json(result).await;
    assert_eq!(res_json["status"].as_str().unwrap(), "unwatched");

    drop(stubr);
    Ok(())
}

async fn create_watched_items(
    logged_in_client: &mut TestHttpClient<
        impl Service<actix_http::Request, Response = RumbaTestResponse, Error = actix_web::Error>,
    >,
) {
    for i in 1..12 {
        let base_url = format!("/api/v1/plus/watching/?url=/en-US/docs/Web/CSS{}", i);
        let payload = json!({
            "title": format!("CSS: Cascading Style Sheets{}", i),
            "path": "this.gets.ignored",
        });
        let res = logged_in_client
            .post(&base_url, None, Some(PostPayload::Json(payload)))
            .await;
        assert_eq!(res.response().status(), 200);
        thread::sleep(Duration::from_millis(10));
    }
}

#[actix_rt::test]
async fn test_watched_item_subscription_limit() -> Result<(), Error> {
    let pool = reset()?;

    let stubr = Stubr::start_blocking_with(
        vec![
            "tests/stubs",
            "tests/test_specific_stubs/collections",
            "tests/test_specific_stubs/watched_items",
        ],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    for i in 1..3 {
        let base_url = format!("/api/v1/plus/watching/?url=/en-US/docs/Web/CSS{}", i);
        let payload = json!({
            "title": format!("CSS: Cascading Style Sheets{}", i),
            "path": "this.gets.ignored",
        });
        let res = logged_in_client
            .post(&base_url, None, Some(PostPayload::Json(payload)))
            .await;
        assert_eq!(res.response().status(), 200);
        thread::sleep(Duration::from_millis(10));
    }

    let mut res = logged_in_client
        .get("/api/v1/plus/watching/?offset=0&limit=10", None)
        .await;

    assert_eq!(res.response().status(), 200);
    let mut res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 2);
    assert!(!res_json["subscription_limit_reached"].as_bool().unwrap());

    //Create one more putting it on the limit

    let payload = json!({
        "title": format!("CSS: Cascading Style Sheets{}", 3),
        "path": "this.gets.ignored",
    });
    let mut base_url = format!("/api/v1/plus/watching/?url=/en-US/docs/Web/CSS{}", 3);
    res = logged_in_client
        .post(&base_url, None, Some(PostPayload::Json(payload.clone())))
        .await;
    assert_eq!(res.response().status(), 200);
    res_json = read_json(res).await;
    //Check limit flag in POST response

    assert!(res_json["subscription_limit_reached"].as_bool().unwrap());
    //Check limit flag in GET
    res = logged_in_client
        .get("/api/v1/plus/watching/?offset=0&limit=10", None)
        .await;

    assert_eq!(res.response().status(), 200);
    res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 3);
    assert!(res_json["subscription_limit_reached"].as_bool().unwrap());

    // Check for 400 if creating new item at the limit

    base_url = format!("/api/v1/plus/watching/?url=/en-US/docs/Web/CSS{}", 4);
    res = logged_in_client
        .post(&base_url, None, Some(PostPayload::Json(payload)))
        .await;
    assert_eq!(res.response().status(), 400);
    res_json = read_json(res).await;
    assert_eq!(res_json["error"].as_str().unwrap(), "max_subscriptions");

    // Check for limit 'false' after deletion of 1

    base_url = format!("/api/v1/plus/watching/?url=/en-US/docs/Web/CSS{}", 3);
    res = logged_in_client
        .post(
            &base_url,
            None,
            Some(PostPayload::Json(json!({"unwatch": true}))),
        )
        .await;

    assert_eq!(res.response().status(), 200);
    res_json = read_json(res).await;
    assert!(!res_json["subscription_limit_reached"].as_bool().unwrap(),);

    drop(stubr);
    Ok(())
}
