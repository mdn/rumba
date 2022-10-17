use actix_http::Request;
use actix_web::cookie::{Cookie, CookieJar, Key};
use actix_web::dev::Service;
use actix_web::test::TestRequest;
use actix_web::{test, Error};
use rumba::settings::SETTINGS;
use std::collections::HashMap;

use reqwest::{Client, Method, StatusCode};

use serde_json::Value;
use url::Url;

use super::RumbaTestResponse;

pub struct TestHttpClient<T: Service<Request, Response = RumbaTestResponse, Error = Error>> {
    pub service: T,
    pub cookies: CookieJar,
}

pub enum PostPayload {
    Json(Value),
    FormData(Value),
}

impl<T: Service<Request, Response = RumbaTestResponse, Error = Error>> TestHttpClient<T> {
    pub async fn new(service: T) -> Self {
        let _stubr_ok = check_stubr_initialized().await;

        let login_req = test::TestRequest::get()
            .uri("/users/fxa/login/authenticate/")
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
        let mut base = test::TestRequest::get().uri(&*format!(
            "/users/fxa/login/callback/?code={:1}&state={:2}",
            "ABC123", state
        ));
        for cookie in cookies {
            base = base.cookie(cookie);
        }

        let mut cookie_jar = CookieJar::new();
        let res = test::call_service(&service, base.to_request()).await;

        let cookies = res.response().cookies();
        for cookie in cookies {
            cookie_jar.add(cookie.into_owned());
        }
        Self {
            service,
            cookies: cookie_jar,
        }
    }

    pub fn with_legacy_session(service: T, id: &'static str) -> Self {
        let mut cookie_jar = CookieJar::new();
        cookie_jar
            .private_mut(&Key::derive_from(&SETTINGS.auth.auth_cookie_key))
            .add(Cookie::new("auth-cookie", id));

        Self {
            service,
            cookies: cookie_jar,
        }
    }

    pub async fn get(
        &mut self,
        uri: &str,
        headers: Option<Vec<(&str, &str)>>,
    ) -> RumbaTestResponse {
        let mut base = test::TestRequest::get().uri(uri);
        base = self.add_cookies_and_headers(headers, base);
        let res = test::call_service(&self.service, base.to_request()).await;
        for cookie in res.response().cookies() {
            self.cookies.add(cookie.into_owned());
        }
        res
    }

    pub async fn post(
        &mut self,
        uri: &str,
        headers: Option<Vec<(&str, &str)>>,
        payload: Option<PostPayload>,
    ) -> RumbaTestResponse {
        let mut base = test::TestRequest::post().uri(uri);
        if let Some(payload) = payload {
            match payload {
                PostPayload::FormData(form) => base = base.set_form(form),
                PostPayload::Json(val) => base = base.set_json(val),
            }
        }

        base = self.add_cookies_and_headers(headers, base);
        let res = test::call_service(&self.service, base.to_request()).await;
        for cookie in res.response().cookies() {
            self.cookies.add(cookie.into_owned());
        }
        res
    }

    pub async fn delete(
        &mut self,
        uri: &str,
        headers: Option<Vec<(&str, &str)>>,
    ) -> RumbaTestResponse {
        let mut base = test::TestRequest::delete().uri(uri);
        base = self.add_cookies_and_headers(headers, base);
        let res = test::call_service(&self.service, base.to_request()).await;
        for cookie in res.response().cookies() {
            self.cookies.add(cookie.into_owned());
        }
        res
    }

    pub async fn trigger_webhook(&self, bearer: &str) -> RumbaTestResponse {
        let req = test::TestRequest::get()
            .uri("/events/fxa")
            .insert_header(("Authorization", format!("Bearer {}", bearer).as_str()))
            .to_request();
        test::call_service(&self.service, req).await
    }

    fn add_cookies_and_headers(
        &self,
        headers: Option<Vec<(&str, &str)>>,
        mut base: TestRequest,
    ) -> TestRequest {
        if let Some(headers) = headers {
            for header in headers {
                base = base.insert_header(header);
            }
        }
        for cookie in self.cookies.iter() {
            base = base.cookie(cookie.clone());
        }
        base
    }
}

pub async fn check_stubr_initialized() -> Result<(), ()> {
    //Hardcoded for now. We will 'always' spin stubr at localhost:4321.
    let res = Client::new()
        .request(Method::GET, "http://localhost:4321/healthz")
        .send()
        .await;
    assert_eq!(res.unwrap().status(), StatusCode::OK);
    Ok(())
}
