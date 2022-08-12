use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use serde_json::{json, Value};

use std::thread;
use std::time::Duration;
use stubr::{Config, Stubr};

// /en-US/docs/Web/CSS -> URL

#[actix_rt::test]
async fn test_create_and_get_collection() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let bookmarked = &collection_json["bookmarked"];
    assert!(!bookmarked.is_null());
    assert_eq!(bookmarked["title"], "CSS: Cascading Style Sheets");
    assert_eq!(bookmarked["url"], "/en-US/docs/Web/CSS");
    assert_eq!(bookmarked["notes"], "Notes notes notes");
    assert_eq!(bookmarked["parents"].as_array().unwrap().len(), 2);
    assert_eq!(bookmarked["parents"][0]["uri"], "/en-US/docs/Web");
    assert_eq!(bookmarked["parents"][0]["title"], "References");
    assert_eq!(bookmarked["parents"][1]["uri"], "/en-US/docs/Web/CSS");
    assert_eq!(bookmarked["parents"][1]["title"], "CSS");

    Ok(())
}

#[actix_rt::test]
async fn test_create_and_get_collection_with_empty_title() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let payload = json!({
        "name": "",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let bookmarked = &collection_json["bookmarked"];
    assert!(!bookmarked.is_null());
    assert_eq!(bookmarked["title"], "CSS: Cascading Style Sheets");
    assert_eq!(bookmarked["url"], "/en-US/docs/Web/CSS");

    Ok(())
}

#[actix_rt::test]
async fn test_create_get_delete_create_collection() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let bookmarked = &collection_json["bookmarked"];
    assert!(!bookmarked.is_null());
    assert_eq!(bookmarked["title"], "CSS: Cascading Style Sheets");
    assert_eq!(bookmarked["url"], "/en-US/docs/Web/CSS");
    assert_eq!(bookmarked["notes"], "Notes notes notes");

    let delete_res = logged_in_client.delete(base_url, None).await;
    assert_eq!(delete_res.status(), 200);
    let try_get_collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(try_get_collection_res).await;
    assert_eq!(collection_json["bookmarked"], Value::Null);

    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let bookmarked = &collection_json["bookmarked"];
    assert_ne!(collection_json["bookmarked"], Value::Null);
    assert_eq!(bookmarked["title"], "CSS: Cascading Style Sheets");
    assert_eq!(bookmarked["url"], "/en-US/docs/Web/CSS");
    assert_eq!(bookmarked["notes"], "Notes notes notes notes");

    Ok(())
}

#[actix_rt::test]
async fn test_pagination_default_sort_by_created() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;

    for i in 1..12 {
        let base_url = format!("/api/v1/plus/collection/?url=/en-US/docs/Web/CSS{}", i);
        let payload = json!({
            "name": format!("CSS: Cascading Style Sheets{}", i),
            "notes": "Notes notes notes",
        });
        logged_in_client
            .post(&base_url, None, Some(PostPayload::FormData(payload)))
            .await;
        thread::sleep(Duration::from_millis(10));
    }

    let base_url = "/api/v1/plus/collection/?limit=5";
    let mut collection_res = logged_in_client.get(base_url, None).await;
    let mut collection_json = read_json(collection_res).await;

    assert_eq!(collection_json["items"].as_array().unwrap().len(), 5);
    let mut items = collection_json["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert!(items[0]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS11"));
    assert!(items[1]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS10"));
    assert!(items[2]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS9"));
    assert!(items[3]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS8"));
    assert!(items[4]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS7"));

    let base_url = "/api/v1/plus/collection/?limit=5&offset=5";
    collection_res = logged_in_client.get(base_url, None).await;
    collection_json = read_json(collection_res).await;
    items = collection_json["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert!(items[0]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS6"));
    assert!(items[1]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS5"));
    assert!(items[2]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS4"));
    assert!(items[3]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS3"));
    assert!(items[4]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS2"));

    let base_url = "/api/v1/plus/collection/?limit=5&offset=10";
    collection_res = logged_in_client.get(base_url, None).await;
    collection_json = read_json(collection_res).await;
    items = collection_json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0]["url"]
        .as_str()
        .unwrap()
        .to_string()
        .ends_with("CSS1"));

    Ok(())
}

#[actix_rt::test]
async fn test_create_fails_404_no_index_found() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS_DEFINITELY_DOESNT_EXIST";
    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 404);

    Ok(())
}

/**
This test creates 2 documents with the same underlying metadata. The second one has a custom_name
added by changing the name in the creation request.
*/
#[actix_rt::test]
async fn test_filters_by_custom_name_over_title() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let mut base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS1";
    let request_no_custom_name = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(request_no_custom_name)),
        )
        .await;

    base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS2";
    let request_custom_name = json!({
        "name": "Crippling Style Shorts",
        "notes": "Notes for CSS2",
    });
    logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(request_custom_name)),
        )
        .await;

    base_url = "/api/v1/plus/collection/?q=Style";

    let mut collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let items = &collection_json["items"];
    assert_eq!(items.as_array().unwrap().len(), 2);

    assert_eq!(items[0]["title"], "Crippling Style Shorts");
    assert_eq!(items[0]["url"], "/en-US/docs/Web/CSS2");
    assert_eq!(items[1]["title"], "CSS: Cascading Style Sheets");
    assert_eq!(items[1]["url"], "/en-US/docs/Web/CSS1");

    // Query the API with the 'custom_name' to ensure it is the only result returned.

    base_url = "/api/v1/plus/collection/?q=Style+Shorts";

    collection_res = logged_in_client.get(base_url, None).await;
    let filtered_collection_json = read_json(collection_res).await;
    let items_filtered = &filtered_collection_json["items"];
    assert_eq!(items_filtered.as_array().unwrap().len(), 1);
    assert_eq!(items_filtered[0]["title"], "Crippling Style Shorts");
    assert_eq!(items_filtered[0]["url"], "/en-US/docs/Web/CSS2");

    Ok(())
}

#[actix_rt::test]
async fn test_query_finds_strings_in_notes() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS1";
    let request_no_custom_name = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(request_no_custom_name)),
        )
        .await;

    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS2";
    let request_custom_name = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "RANDOM",
    });
    logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(request_custom_name)),
        )
        .await;

    let base_url = "/api/v1/plus/collection/?q=RANDOM";

    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let items = &collection_json["items"];
    assert_eq!(items.as_array().unwrap().len(), 1);

    assert_eq!(items[0]["title"], "CSS: Cascading Style Sheets");
    assert_eq!(items[0]["url"], "/en-US/docs/Web/CSS2");
    assert_eq!(items[0]["notes"], "RANDOM");
    Ok(())
}

#[actix_rt::test]
async fn test_delete_collection() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;
    let bookmarked = &collection_json["bookmarked"];
    assert!(!bookmarked.is_null());
    let delete_res = logged_in_client.delete(base_url, None).await;
    assert_eq!(delete_res.status(), 200);
    let try_get_collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(try_get_collection_res).await;
    assert!(collection_json["bookmarked"].is_null());
    Ok(())
}

#[actix_rt::test]
async fn test_undelete_collection() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;
    let bookmarked = &collection_json["bookmarked"];
    assert!(!bookmarked.is_null());
    let delete_res = logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({"delete": "true"}))),
        )
        .await;
    assert_eq!(delete_res.status(), 200);
    let try_get_collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(try_get_collection_res).await;
    assert!(collection_json["bookmarked"].is_null());
    let delete_res = logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({"delete": "false"}))),
        )
        .await;
    assert_eq!(delete_res.status(), 200);
    let try_get_collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(try_get_collection_res).await;
    assert!(!collection_json["bookmarked"].is_null());
    Ok(())
}

#[actix_rt::test]
async fn test_delete_collection_via_post() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let payload = json!({
        "name": "CSS: Cascading Style Sheets",
        "notes": "Notes notes notes",
    });
    let create_res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;
    let bookmarked = &collection_json["bookmarked"];
    assert!(!bookmarked.is_null());
    let delete_res = logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({"delete": "true"}))),
        )
        .await;
    assert_eq!(delete_res.status(), 200);
    let try_get_collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(try_get_collection_res).await;
    assert!(collection_json["bookmarked"].is_null());
    Ok(())
}

#[actix_rt::test]
async fn test_collection_subscription_limits() -> Result<(), Error> {
    reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec![
            "tests/stubs",
            "tests/test_specific_stubs/collections",
            "tests/test_specific_stubs/collections_core_user",
        ],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;

    for i in 1..5 {
        let base_url = format!("/api/v1/plus/collection/?url=/en-US/docs/Web/CSS{}", i);
        let payload = json!({
            "name": format!("CSS: Cascading Style Sheets{}", i),
            "notes": "Notes notes notes",
        });
        logged_in_client
            .post(&base_url, None, Some(PostPayload::FormData(payload)))
            .await;
        thread::sleep(Duration::from_millis(10));
    }

    let mut base_url = "/api/v1/plus/collection/?limit=5";
    let mut res = logged_in_client.get(base_url, None).await;
    let mut collection_json = read_json(res).await;

    assert_eq!(collection_json["items"].as_array().unwrap().len(), 4);
    assert!(!collection_json["subscription_limit_reached"]
        .as_bool()
        .unwrap());

    //Assert that on creating one more limit is reached
    base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS5";
    let payload = json!({
        "name": format!("CSS: Cascading Style Sheets{}", 5),
        "notes": "Notes notes notes",
    });
    res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(res.status(), 201);
    collection_json = read_json(res).await;
    assert!(collection_json["subscription_limit_reached"]
        .as_bool()
        .unwrap());

    //Assert creating new one is 400
    base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS6";
    let payload = json!({
        "name": format!("CSS: Cascading Style Sheets{}", 6),
        "notes": "Notes notes notes",
    });
    res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(res.status(), 400);
    collection_json = read_json(res).await;
    assert_eq!(
        collection_json["error"].as_str().unwrap(),
        "max_subscriptions"
    );

    //Assert updating existing is success and limit still reached
    base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS5";
    let payload = json!({
        "name": format!("Updated CSS: Cascading Style Sheets{}", 5),
        "notes": "New notes",
    });
    res = logged_in_client
        .post(base_url, None, Some(PostPayload::FormData(payload)))
        .await;
    assert_eq!(res.status(), 201);
    collection_json = read_json(res).await;
    assert!(collection_json["subscription_limit_reached"]
        .as_bool()
        .unwrap());

    // Assert deleting one is success and subscription limit no more reached
    res = logged_in_client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({"delete": true}))),
        )
        .await;
    assert_eq!(res.status(), 200);
    collection_json = read_json(res).await;
    assert!(!collection_json["subscription_limit_reached"]
        .as_bool()
        .unwrap());
    Ok(())
}
