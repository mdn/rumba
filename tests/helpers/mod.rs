use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use actix_http::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::ServiceResponse;
use actix_web::test;
use anyhow::{anyhow, Error};
use serde_json::Value;

pub mod api_assertions;
pub mod app;
pub mod db;
pub mod http_client;
pub mod identity;

pub type RumbaTestResponse = ServiceResponse<EitherBody<EitherBody<BoxBody>>>;

pub async fn read_json<B: MessageBody + Unpin>(res: ServiceResponse<B>) -> Value {
    serde_json::from_slice(test::read_body(res).await.as_ref()).unwrap()
}

pub fn wait_for_stubr() -> Result<(), Error> {
    let stream = TcpStream::connect_timeout(&"127.0.0.1:4321".parse()?, Duration::from_millis(10))
        .map_err(|_| anyhow!("strubr not ready after 10ms"))?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}
