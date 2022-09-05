use std::time::Duration;

use actix_http::body::{BoxBody, EitherBody, MessageBody};
use actix_rt::{net::TcpStream, time::timeout};
use actix_web::dev::ServiceResponse;
use actix_web::test;
use anyhow::{anyhow, Error};
use chrono::{DateTime, NaiveDateTime, Utc};
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

pub async fn wait_for_stubr() -> Result<(), Error> {
    timeout(Duration::from_millis(1_000), async {
        TcpStream::connect(("127.0.0.1", 4321))
            .await
            .map_err(|_| anyhow!("strubr not ready after 1000ms"))?;
        Ok::<(), Error>(())
    })
    .await??;

    Ok(())
}

pub fn naive_to_date_time_utc(updated: NaiveDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(updated, Utc)
}
