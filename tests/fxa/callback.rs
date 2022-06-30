use actix_web::test;

use anyhow::Error;
use std::collections::HashMap;

use url::Url;

use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn basic() -> Result<(), Error> {
    reset()?;

    let app = test_app_with_login().await.unwrap();
    let app = test::init_service(app).await;

    let login_req = test::TestRequest::get()
        .uri("/users/fxa/login/authenticate/")
        .to_request();
    let login_res = test::call_service(&app, login_req).await;

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

    let mut base = test::TestRequest::get().uri(&*format!(
        "/users/fxa/login/callback/?code={:1}&state={:2}",
        "ABC123", state
    ));
    for cookie in cookies {
        base = base.cookie(cookie);
    }

    let res = test::call_service(&app, base.to_request()).await;
    assert!(res.status().is_redirection());
    assert_eq!(res.headers().get("Location").unwrap(), "/");

    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn next() -> Result<(), Error> {
    reset()?;

    let app = test_app_with_login().await.unwrap();
    let app = test::init_service(app).await;

    let login_req = test::TestRequest::get()
        .uri("/users/fxa/login/authenticate/?next=/foo")
        .to_request();
    let login_res = test::call_service(&app, login_req).await;

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

    let mut base = test::TestRequest::get().uri(&*format!(
        "/users/fxa/login/callback/?code={:1}&state={:2}",
        "ABC123", state
    ));
    for cookie in cookies {
        base = base.cookie(cookie);
    }

    let res = test::call_service(&app, base.to_request()).await;
    assert!(res.status().is_redirection());
    assert_eq!(res.headers().get("Location").unwrap(), "/foo");

    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn no_prompt() -> Result<(), Error> {
    reset()?;

    let app = test_app_with_login().await.unwrap();
    let app = test::init_service(app).await;

    let login_req = test::TestRequest::get()
        .uri("/users/fxa/login/authenticate/no-prompt/?next=/foo&email=foo@bar.com")
        .to_request();
    let login_res = test::call_service(&app, login_req).await;

    assert!(login_res.status().is_redirection());
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

    let prompt = params.get("prompt").to_owned().unwrap().clone();
    let login_hint = params.get("login_hint").to_owned().unwrap().clone();
    assert_eq!(prompt, "none");
    assert_eq!(login_hint, "foo@bar.com");

    let state = params.get("state").to_owned().unwrap().clone();

    let mut base = test::TestRequest::get().uri(&*format!(
        "/users/fxa/login/callback/?code={:1}&state={:2}",
        "ABC123", state
    ));
    for cookie in cookies {
        base = base.cookie(cookie);
    }

    let res = test::call_service(&app, base.to_request()).await;
    assert!(res.status().is_redirection());
    assert_eq!(res.headers().get("Location").unwrap(), "/foo");

    Ok(())
}
