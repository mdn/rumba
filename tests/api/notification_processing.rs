use actix_web::test;
use anyhow::Error;
use serde_json::{json, Value};
use stubr::{Config, Stubr};

use crate::helpers::db::reset;
use crate::helpers::wait_for_stubr;
use crate::helpers::{
    app::test_app_with_login,
    http_client::{PostPayload, TestHttpClient},
    read_json,
};

#[actix_rt::test]
async fn test_receive_notification_subscribed_top_level() -> Result<(), Error> {
    let pool = reset()?;
    let stubr = Stubr::start_blocking_with(
        vec![
            "tests/stubs",
            "tests/test_specific_stubs/notifications_processing",
        ],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Given a user is watching API/Navigator

    let mut base_url = "/api/v1/plus/watching/?url=/en-US/docs/Web/API/Navigator";
    let mut payload = json!({
        "title": "Navigator",
        "path": "this.gets.ignored",
    });
    let mut res = logged_in_client
        .post(base_url, None, Some(PostPayload::Json(payload)))
        .await;
    assert_eq!(res.response().status(), 200);

    //When notifications are triggered for API/Navigator/vibrate and API/Navigator/connection
    base_url = "/admin-api/update/";
    payload = json!({"filename" : "bcd-changes-test.json"});
    res = logged_in_client
        .post(
            base_url,
            Some(vec![("Authorization", "Bearer TEST_TOKEN")]),
            Some(PostPayload::Json(payload)),
        )
        .await;
    assert_eq!(res.response().status(), 200);

    //Then they should receive them as API/Navigator is the parent
    base_url = "/api/v1/plus/notifications/";
    res = logged_in_client.get(base_url, None).await;
    assert_eq!(res.response().status(), 200);

    let notifications_json = read_json(res).await;
    let notifications = notifications_json["items"].as_array().unwrap();
    assert_eq!(notifications.len(), 4);
    //Should all be 'Compat'
    assert!(!notifications.iter().any(|val| val["type"] == "content"));
    //Can't guarentee creation time so Alphabetically :

    // Compatibility subfeature added
    // In development in Nightly 99
    // Removed from Firefox for Android 99
    // Supported in Chrome 102, Chrome Android 102 and WebView Android 102

    let mut sorted: Vec<Value> = notifications
        .iter()
        .map(|val| val.to_owned())
        .collect::<Vec<Value>>();
    sorted.sort_by(|a, b| {
        a["text"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .cmp(&b["text"].as_str().unwrap().to_lowercase())
    });

    //Compatiblity SubFeature added notification
    assert_eq!(
        sorted[0]["text"].as_str().unwrap(),
        "Compatibility subfeature added"
    );
    assert_eq!(
        sorted[0]["title"].as_str().unwrap(),
        "Navigator.crazy_new_subfeature"
    );

    //Preview notification
    assert_eq!(
        sorted[1]["text"].as_str().unwrap(),
        "In development in Nightly 99"
    );

    assert_eq!(
        sorted[1]["title"].as_str().unwrap(),
        "Navigator.new_thing_for_preview"
    );
    // Api removed
    assert_eq!(
        sorted[2]["text"].as_str().unwrap(),
        "Removed from Firefox for Android 99"
    );
    assert_eq!(
        sorted[2]["title"].as_str().unwrap(),
        "Navigator.very_unstable_feature"
    );
    // Stable added
    assert_eq!(
        sorted[3]["text"].as_str().unwrap(),
        "Supported in Chrome 102, Chrome Android 102 and WebView Android 102"
    );
    assert_eq!(sorted[3]["title"].as_str().unwrap(), "Navigator.vibrate");

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_receive_notification_subscribed_specific_path() -> Result<(), Error> {
    let pool = reset()?;
    let stubr = Stubr::start_blocking_with(
        vec![
            "tests/stubs",
            "tests/test_specific_stubs/notifications_processing",
        ],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Given a user is watching API/Navigator/vibrate

    let mut base_url = "/api/v1/plus/watching/?url=/en-US/docs/Web/API/Navigator/vibrate";
    let mut payload = json!({
        "title": "Navigator",
        "path": "this.gets.ignored",
    });
    let mut res = logged_in_client
        .post(base_url, None, Some(PostPayload::Json(payload)))
        .await;
    assert_eq!(res.response().status(), 200);

    //When notifications are triggered for API/Navigator/vibrate
    base_url = "/admin-api/update/";
    payload = json!({"filename" : "bcd-changes-test.json"});
    res = logged_in_client
        .post(
            base_url,
            Some(vec![("Authorization", "Bearer TEST_TOKEN")]),
            Some(PostPayload::Json(payload)),
        )
        .await;
    assert_eq!(res.response().status(), 200);

    //Then they should receive a stable added and a content notification
    base_url = "/api/v1/plus/notifications/";
    res = logged_in_client.get(base_url, None).await;
    assert_eq!(res.response().status(), 200);

    let notifications_json = read_json(res).await;
    let notifications = notifications_json["items"].as_array().unwrap();
    assert_eq!(notifications.len(), 2);

    let mut sorted: Vec<Value> = notifications
        .iter()
        .map(|val| val.to_owned())
        .collect::<Vec<Value>>();
    sorted.sort_by(|a, b| {
        a["text"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .cmp(&b["text"].as_str().unwrap().to_lowercase())
    });
    assert_eq!(
        sorted[0]["text"].as_str().unwrap(),
        "Page updated (see PR!https://github.com/mdn/content/pull/1337!mdn/content!!)"
    );
    assert_eq!(sorted[0]["title"].as_str().unwrap(), "Navigator.vibrate()");
    // Content added
    assert_eq!(
        sorted[1]["text"].as_str().unwrap(),
        "Supported in Chrome 102, Chrome Android 102 and WebView Android 102"
    );
    assert_eq!(sorted[1]["title"].as_str().unwrap(), "vibrate");
    // Stable added
    assert_eq!(
        sorted[1]["text"].as_str().unwrap(),
        "Supported in Chrome 102, Chrome Android 102 and WebView Android 102"
    );

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_receive_notification_unknown() -> Result<(), Error> {
    let pool = reset()?;
    let stubr = Stubr::start_blocking_with(
        vec![
            "tests/stubs",
            "tests/test_specific_stubs/notifications_processing",
        ],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Given a user is watching API/Navigator

    let mut base_url = "/api/v1/plus/watching/?url=/en-US/docs/Web/API/Navigator";
    let mut payload = json!({
        "title": "Navigator",
        "path": "this.gets.ignored",
    });
    let mut res = logged_in_client
        .post(base_url, None, Some(PostPayload::Json(payload)))
        .await;
    assert_eq!(res.response().status(), 200);

    //When notifications are triggered for API/Navigator/vibrate and API/Navigator/connection
    base_url = "/admin-api/update/";
    payload = json!({"filename" : "bcd-changes-test-with-unknown.json"});
    res = logged_in_client
        .post(
            base_url,
            Some(vec![("Authorization", "Bearer TEST_TOKEN")]),
            Some(PostPayload::Json(payload)),
        )
        .await;
    assert_eq!(res.response().status(), 200);

    //Then they should receive them as API/Navigator is the parent
    base_url = "/api/v1/plus/notifications/";
    res = logged_in_client.get(base_url, None).await;
    assert_eq!(res.response().status(), 200);

    let notifications_json = read_json(res).await;
    let notifications = notifications_json["items"].as_array().unwrap();
    assert_eq!(notifications.len(), 1);

    assert_eq!(
        notifications[0]["text"].as_str().unwrap(),
        "Supported in Firefox 3.5"
    );
    assert_eq!(
        notifications[0]["title"].as_str().unwrap(),
        "Navigator.vibrate_more"
    );
    drop(stubr);
    Ok(())
}
