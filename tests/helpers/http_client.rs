use actix_http::body::{BoxBody, EitherBody};
use actix_http::Request;
use actix_web::cookie::CookieJar;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::test::TestRequest;
use actix_web::{test, Error};
use std::collections::HashMap;

use reqwest::{Client, Method, StatusCode};

use serde_json::Value;
use url::Url;

pub struct TestHttpClient<
    T: Service<Request, Response = ServiceResponse<EitherBody<BoxBody>>, Error = Error>,
> {
    service: T,
    cookies: CookieJar,
}

type FormData = Vec<(String, String)>;

pub enum PostPayload {
    Json(Value),
    FormData(FormData),
}

impl<T: Service<Request, Response = ServiceResponse<EitherBody<BoxBody>>, Error = Error>>
    TestHttpClient<T>
{
    pub async fn new(service: T) -> Self {
        let _stubr_ok = check_stubr_initialized().await;

        let login_req = test::TestRequest::get()
            .uri("/users/fxa/login/authenticate")
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

    pub async fn get(
        &mut self,
        uri: String,
        headers: Option<Vec<(&str, &str)>>,
    ) -> ServiceResponse<EitherBody<BoxBody>> {
        let mut base = test::TestRequest::get().uri(&*uri);
        base = self.add_cookies_and_headers(headers, base);
        let res = test::call_service(&self.service, base.to_request()).await;
        for cookie in res.response().cookies() {
            self.cookies.add(cookie.into_owned());
        }
        res
    }

    pub(crate) async fn post(
        &mut self,
        uri: String,
        headers: Option<Vec<(&str, &str)>>,
        payload: PostPayload,
    ) -> ServiceResponse<EitherBody<BoxBody>> {
        let mut base = test::TestRequest::post().uri(&*uri);
        match payload {
            PostPayload::FormData(form) => base = base.set_form(form),
            PostPayload::Json(val) => base = base.set_json(val),
        }
        base = self.add_cookies_and_headers(headers, base);
        let res = test::call_service(&self.service, base.to_request()).await;
        for cookie in res.response().cookies() {
            self.cookies.add(cookie.into_owned());
        }
        res
    }

    fn add_cookies_and_headers(
        &self,
        headers: Option<Vec<(&str, &str)>>,
        mut base: TestRequest,
    ) -> TestRequest {
        match headers {
            Some(headers) => {
                for header in headers {
                    base = base.insert_header(header);
                }
            }
            None => (),
        }
        for cookie in self.cookies.iter() {
            base = base.cookie(cookie.clone());
        }
        return base;
    }
}

async fn check_stubr_initialized() -> Result<(), ()> {
    //Hardcoded for now. We will 'always' spin stubr at localhost:4321.
    let res = Client::new()
        .request(Method::GET, "http://localhost:4321/healthz")
        .send()
        .await;
    assert_eq!(res.unwrap().status(), StatusCode::OK);
    Ok(())
}
