use std::time::Duration;

use actix_http::body::{BoxBody, EitherBody, MessageBody};
use actix_rt::{
    net::TcpStream,
    time::{sleep, timeout},
};
use actix_web::dev::ServiceResponse;
use actix_web::test;
use anyhow::{anyhow, Error};
use serde_json::Value;

pub mod api_assertions;
pub mod app;
pub mod db;
pub mod http_client;

pub type RumbaTestResponse = ServiceResponse<EitherBody<BoxBody>>;

pub async fn read_json<B: MessageBody + Unpin>(res: ServiceResponse<B>) -> Value {
    serde_json::from_slice(test::read_body(res).await.as_ref()).unwrap()
}

pub async fn wait_for_stubr() -> Result<(), Error> {
    timeout(Duration::from_millis(10_000), async {
        while let Err(_e) = TcpStream::connect(("127.0.0.1", 4321)).await {
            sleep(Duration::from_millis(100)).await;
        }
        Ok::<(), Error>(())
    })
    .await
    .map_err(|_| anyhow!("strubr not ready after 10,000ms"))??;

    Ok(())
}
