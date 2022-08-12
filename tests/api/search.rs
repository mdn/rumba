use crate::helpers::read_json;
use crate::helpers::{app::test_app_only_search, wait_for_stubr};
use actix_web::{http::header, test};
use anyhow::Error;
use stubr::{Config, Stubr};

async fn do_request(
    path: &str,
) -> Result<
    actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
    Error,
> {
    let _stubr = Stubr::start_blocking_with(
        vec!["tests/test_specific_stubs/search"],
        Config {
            port: Some(4321),
            verbose: Some(true),
            global_delay: None,
            latency: None,
        },
    );
    wait_for_stubr()?;
    let app = test_app_only_search().await;
    let service = test::init_service(app).await;
    let request = test::TestRequest::get().uri(path).to_request();
    Ok(test::call_service(&service, request).await)
}

#[actix_rt::test]
async fn test_basic() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 52_i64);
    assert_eq!(json["metadata"]["total"]["value"], 745_i64);
    assert_eq!(json["metadata"]["total"]["relation"], "eq");
    assert_eq!(json["metadata"]["size"], 10_i64);
    assert_eq!(json["metadata"]["page"], 1_i64);
    assert_eq!(json["suggestions"].as_array().unwrap().len(), 0);
    assert_eq!(json["documents"].as_array().unwrap().len(), 10);
    assert_eq!(json["documents"][0]["mdn_url"], "/en-US/docs/Mozilla");
    assert_eq!(json["documents"][0]["score"], 53.950974_f64);
    assert_eq!(json["documents"][0]["title"], "Mozilla");
    assert_eq!(json["documents"][0]["locale"], "en-us");
    assert_eq!(json["documents"][0]["slug"], "mozilla");
    assert_eq!(json["documents"][0]["popularity"], 0.007_f64);
    assert_eq!(json["documents"][0]["summary"], "The articles below include content about downloading and building Mozilla code. In addition, you'll find helpful articles about how the code works, how to build add-ons for Mozilla applications and the like.");
    assert_eq!(
        json["documents"][0]["highlight"]["body"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(json["documents"][0]["highlight"]["body"][0], "The articles below include content about downloading and building <mark>Mozilla</mark> code.");
    assert_eq!(
        json["documents"][0]["highlight"]["title"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        json["documents"][0]["highlight"]["title"][0],
        "<mark>Mozilla</mark>"
    );
    Ok(())
}

#[actix_rt::test]
async fn test_sort_relevance() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&sort=relevance&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 87_i64);
    assert_eq!(json["documents"][0]["title"], "Mozilla");
    Ok(())
}

#[actix_rt::test]
async fn test_sort_popularity() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&sort=popularity&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 145_i64);
    assert_eq!(json["documents"][0]["title"], "Specificity");
    Ok(())
}

#[actix_rt::test]
async fn test_sort_invalid() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&sort=foobar&locale=en-US").await?;

    assert!(search.status().is_client_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    let json = read_json(search).await;
    assert!(json["errors"]["sort"][0]["message"].is_string());
    assert_eq!(json["errors"]["sort"][0]["code"], "invalid");
    Ok(())
}

#[actix_rt::test]
async fn test_locale_multiple() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&locale=fr&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 39_i64);
    assert_eq!(json["documents"][0]["locale"], "en-us");
    assert_eq!(json["documents"][1]["locale"], "fr");
    Ok(())
}

#[actix_rt::test]
async fn test_locale_invalid() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&locale=foobar").await?;

    assert!(search.status().is_client_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    let json = read_json(search).await;
    assert!(json["errors"]["locale"][0]["message"].is_string());
    assert_eq!(json["errors"]["locale"][0]["code"], "invalid");
    Ok(())
}

#[actix_rt::test]
async fn test_locale_none() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 52_i64);
    assert_eq!(json["metadata"]["total"]["value"], 745_i64);
    Ok(())
}

#[actix_rt::test]
async fn test_page_2() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&page=2&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 119_i64);
    assert_eq!(json["metadata"]["page"], 2_i64);
    assert_eq!(json["metadata"]["size"], 10_i64);
    assert_eq!(json["documents"].as_array().unwrap().len(), 10);
    assert_eq!(
        json["documents"][0]["title"],
        "Proxy Auto-Configuration (PAC) file"
    );
    Ok(())
}

#[actix_rt::test]
async fn test_page_invalid() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&page=foobar&locale=en-US").await?;

    assert!(search.status().is_client_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    let json = read_json(search).await;
    assert!(json["errors"]["page"][0]["message"].is_string());
    assert_eq!(json["errors"]["page"][0]["code"], "invalid");
    Ok(())
}

#[actix_rt::test]
async fn test_page_too_small() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&page=0&locale=en-US").await?;

    assert!(search.status().is_client_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    let json = read_json(search).await;
    assert!(json["errors"]["page"][0]["message"].is_string());
    assert_eq!(json["errors"]["page"][0]["code"], "invalid");
    Ok(())
}

#[actix_rt::test]
async fn test_page_too_big() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=mozilla&page=11&locale=en-US").await?;

    assert!(search.status().is_client_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    let json = read_json(search).await;
    assert!(json["errors"]["page"][0]["message"].is_string());
    assert_eq!(json["errors"]["page"][0]["code"], "invalid");
    Ok(())
}

#[actix_rt::test]
async fn test_suggestion() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=foobar&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 64_i64);
    assert_eq!(json["suggestions"][0]["text"], "footer");
    assert_eq!(json["suggestions"][0]["total"]["value"], 142_i64);
    assert_eq!(json["suggestions"][0]["total"]["relation"], "eq");
    Ok(())
}

#[actix_rt::test]
async fn test_no_results() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=veryspecificquery&locale=en-US").await?;

    assert!(search.status().is_success());
    assert_eq!(
        search.headers().get(header::CACHE_CONTROL).unwrap(),
        "max-age=43200"
    );

    let json = read_json(search).await;
    assert_eq!(json["metadata"]["took_ms"], 98_i64);
    assert_eq!(json["metadata"]["total"]["value"], 0_i64);
    assert_eq!(json["metadata"]["total"]["relation"], "eq");
    assert_eq!(json["documents"].as_array().unwrap().len(), 0);
    Ok(())
}

#[actix_rt::test]
async fn test_elastic_error() -> Result<(), Error> {
    let search = do_request("/api/v1/search?q=closedindex&locale=en-US").await?;

    assert!(search.status().is_server_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    Ok(())
}

#[actix_rt::test]
async fn test_query_too_big() -> Result<(), Error> {
    let search = do_request(&format!(
        "/api/v1/search?q={}&locale=en-US",
        str::repeat("a", 201)
    ))
    .await?;

    assert!(search.status().is_client_error());
    assert!(!search.headers().contains_key(header::CACHE_CONTROL));

    let json = read_json(search).await;
    assert!(json["errors"]["q"][0]["message"].is_string());
    assert_eq!(json["errors"]["q"][0]["code"], "invalid");
    Ok(())
}
