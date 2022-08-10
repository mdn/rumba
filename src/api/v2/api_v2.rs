use crate::api::user_middleware::AddUser;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

use super::multiple_collections::{
    add_collection_item_to_collection, create_multiple_collection, get_collection_by_id,
    get_collections, modify_collection_item_in_collection, remove_collection_item_from_collection,
};

pub fn api_v2_service() -> impl HttpServiceFactory {
    web::scope("/api/v2")
        .wrap(AddUser)
        .service(
            web::resource("/collections/")
                .route(web::get().to(get_collections))
                .route(web::post().to(create_multiple_collection)),
        )
        .service(web::resource("/collections/{id}/").route(web::get().to(get_collection_by_id)))
        .service(
            web::resource("/collections/{id}/items/")
                .route(web::post().to(add_collection_item_to_collection)),
        )
        .service(
            web::resource("/collections/{id}/items/{item_id}")
                .route(web::post().to(remove_collection_item_from_collection))
                .route(web::post().to(modify_collection_item_in_collection)),
        )
}
