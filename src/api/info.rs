use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
struct Info {
    version: &'static str,
}

const INFO: Info = Info {
    version: env!("CARGO_PKG_VERSION"),
};

pub async fn information() -> HttpResponse {
    HttpResponse::Ok().json(INFO)
}
