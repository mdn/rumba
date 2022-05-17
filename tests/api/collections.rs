use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::read_json;
use actix_web::test;
use anyhow::Error;

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

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url = "/api/v1/collections?url=/en-US/docs/Web/CSS".to_string();
    let payload = vec![
        (
            "name".to_string(),
            "CSS: Cascading Style Sheets".to_string(),
        ),
        ("notes".to_string(), "Notes notes notes".to_string()),
    ];
    let create_res = logged_in_client
        .post(base_url.clone(), None, PostPayload::FormData(payload))
        .await;
    assert_eq!(create_res.status(), 201);
    let collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let bookmarked = &collection_json["bookmarked"];
    assert_eq!(bookmarked.is_null(), false);
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

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;

    let mut logged_in_client = TestHttpClient::new(service).await;

    for i in 1..12 {
        let base_url = "/api/v1/collections?url=/en-US/docs/Web/CSS".to_string() + &i.to_string();
        let payload = vec![
            (
                "name".to_string(),
                "CSS: Cascading Style Sheets".to_string() + &i.to_string(),
            ),
            ("notes".to_string(), "Notes notes notes".to_string()),
        ];
        logged_in_client
            .post(base_url.clone(), None, PostPayload::FormData(payload))
            .await;
        thread::sleep(Duration::from_millis(10));
    }

    let mut base_url = "/api/v1/collections?limit=5".to_string();
    let mut collection_res = logged_in_client.get(base_url, None).await;
    let mut collection_json = read_json(collection_res).await;

    assert_eq!(collection_json["items"].as_array().unwrap().len(), 5);
    println!("{:?}", collection_json["items"].as_array());
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

    base_url = "/api/v1/collections?limit=5&offset=5".to_string();
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

    let base_url = "/api/v1/collections?limit=5&offset=10".to_string();
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

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let base_url =
        "/api/v1/collections?url=/en-US/docs/Web/CSS_DEFINITELY_DOESNT_EXIST".to_string();
    let payload = vec![
        (
            "name".to_string(),
            "CSS: Cascading Style Sheets".to_string(),
        ),
        ("notes".to_string(), "Notes notes notes".to_string()),
    ];
    let create_res = logged_in_client
        .post(base_url.clone(), None, PostPayload::FormData(payload))
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

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let mut base_url = "/api/v1/collections?url=/en-US/docs/Web/CSS1".to_string();
    let request_no_custom_name = vec![
        (
            "name".to_string(),
            "CSS: Cascading Style Sheets".to_string(),
        ),
        ("notes".to_string(), "Notes notes notes".to_string()),
    ];
    logged_in_client
        .post(
            base_url.clone(),
            None,
            PostPayload::FormData(request_no_custom_name),
        )
        .await;

    base_url = "/api/v1/collections?url=/en-US/docs/Web/CSS2".to_string();
    let request_custom_name = vec![
        ("name".to_string(), "Crippling Style Shorts".to_string()),
        ("notes".to_string(), "Notes for CSS2".to_string()),
    ];
    logged_in_client
        .post(
            base_url.clone(),
            None,
            PostPayload::FormData(request_custom_name),
        )
        .await;

    base_url = "/api/v1/collections?q=Style".to_string();

    let mut collection_res = logged_in_client.get(base_url, None).await;
    let collection_json = read_json(collection_res).await;

    let items = &collection_json["items"];
    assert_eq!(items.as_array().unwrap().len(), 2);

    assert_eq!(items[0]["title"], "Crippling Style Shorts");
    assert_eq!(items[0]["url"], "/en-US/docs/Web/CSS2");
    assert_eq!(items[1]["title"], "CSS: Cascading Style Sheets");
    assert_eq!(items[1]["url"], "/en-US/docs/Web/CSS1");

    // Query the API with the 'custom_name' to ensure it is the only result returned.

    base_url = "/api/v1/collections?q=Style+Shorts".to_string();

    collection_res = logged_in_client.get(base_url, None).await;
    let filtered_collection_json = read_json(collection_res).await;
    let items_filtered = &filtered_collection_json["items"];
    assert_eq!(items_filtered.as_array().unwrap().len(), 1);
    assert_eq!(items_filtered[0]["title"], "Crippling Style Shorts");
    assert_eq!(items_filtered[0]["url"], "/en-US/docs/Web/CSS2");

    Ok(())
}
