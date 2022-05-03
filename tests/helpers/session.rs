use std::collections::HashMap;
use actix_http::body::{BoxBody, EitherBody};
use actix_web::{App, HttpResponse, test};
use actix_web::cookie::{Cookie, CookieJar};
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::test::TestRequest;
use anyhow::Error;
use reqwest::{Client, Method, StatusCode};
use url::Url;

type AppType = dyn ServiceFactory<
    ServiceRequest,
    Response = ServiceResponse<EitherBody<BoxBody>>,
    Error = Error,
    Config = (),
    InitError = (),
    Service = (),
    Future = ()>;

pub struct TestHttpClient<T: ServiceFactory<
    ServiceRequest,
    Response = ServiceResponse<EitherBody<BoxBody>>,
    Error = Error,
    Config = (),
    InitError = (),
    Service = (),
    Future = ()>> {
    app: App<T>,
    cookies: CookieJar,
}

impl <T: ServiceFactory<
    ServiceRequest,
    Response = ServiceResponse<EitherBody<BoxBody>>,
    Error = Error,
    Config = (),
    InitError = (),
    Service = (),
    Future = ()>> TestHttpClient <T> {
    pub async fn new(app: App<impl ServiceFactory<
        ServiceRequest,
        Response=ServiceResponse<EitherBody<BoxBody>>,
        Error=Error,
        Config=(),
        InitError=Error,
        Service=(),
        Future=()>>) -> Self {
        check_stubr_initialized();


        let login_req = test::TestRequest::get()
            .uri("/users/fxa/login/authenticate")
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
        let cookies = res.response().cookies().clone();




        // let cookies = get_logged_in_session_cookies(&app).await?;
        let mut cookieJar = CookieJar::new();
        for cookie in cookies {
            cookieJar.add(cookie.clone());
        }
        Self { app, cookies: cookieJar }
    }

    async fn get(mut self, uri: String) -> HttpResponse {
        let mut base = test::TestRequest::get().uri(&*uri);
        for cookie in self.cookies.iter() {
            base = base.cookie(cookie.clone());
        }
        let res: HttpResponse = test::call_service(&self.app, base.to_request()).await;
        for cookie in res.cookies() {
            self.cookies.add(cookie.clone());
        }
        res
    }
}

async fn check_stubr_initialized() {
    //Hardcoded for now. We will 'always' spin stubr at localhost:4321.
    let res = Client::new().request(Method::GET, "http://localhost:4321/healthz").send().await?;
    assert_eq!(res.status(), StatusCode::OK);
}

// async fn get_logged_in_session_cookies (app: App<T>) -> Vec<Cookie<'_>> {
//     let login_req = test::TestRequest::get()
//         .uri("/users/fxa/login/authenticate")
//         .to_request();
//     let login_res = test::call_service(&app, login_req).await;
//
//     let location_header = login_res
//         .response()
//         .headers()
//         .get("Location")
//         .unwrap()
//         .to_str()
//         .unwrap();
//     let cookies = login_res.response().cookies();
//
//     let params: HashMap<_, _> = Url::parse(location_header)
//         .unwrap()
//         .query_pairs()
//         .into_owned()
//         .collect();
//     let state = params.get("state").to_owned().unwrap().clone();
//     let mut base = test::TestRequest::get().uri(&*format!(
//         "/users/fxa/login/callback/?code={:1}&state={:2}",
//         "ABC123", state
//     ));
//     for cookie in cookies {
//         base = base.cookie(cookie);
//     }
//     let res = test::call_service(&app, base.to_request()).await;
//     res.response().cookies().clone()
// }