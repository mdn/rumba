use actix_http::body::MessageBody;
use actix_web::dev::ServiceResponse;
use actix_web::test;
use serde_json::Value;

pub mod api_assertions;
pub mod app;
pub mod db;
pub mod http_client;
pub mod identity;

pub async fn read_json<B: MessageBody + Unpin>(res: ServiceResponse<B>) -> Value {
    serde_json::from_slice(test::read_body(res).await.as_ref()).unwrap()
}
