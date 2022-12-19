use actix_web::dev::HttpServiceFactory;
use actix_web::web;

use crate::db::v2::synchronize_bcd_updates_db::update_bcd;

use super::{
    multiple_collections::{
        add_collection_item_to_collection, create_multiple_collection, delete_collection,
        get_collection_by_id, get_collection_item_in_collection_by_id, get_collections,
        lookup_collections_containing_article, modify_collection,
        modify_collection_item_in_collection, remove_collection_item_from_collection,
    },
    updates::get_updates,
};

pub fn api_v2_service() -> impl HttpServiceFactory {
    web::scope("/api/v2")
        .service(
            web::resource("/updates/")
                .route(web::get().to(get_updates))
                .route(web::post().to(update_bcd))
                .route(web::delete().to(remove_collection_item_from_collection)),
        )
        .service(
            web::resource("/collections/")
                .route(web::get().to(get_collections))
                .route(web::post().to(create_multiple_collection)),
        )
        .service(
            web::resource("/collections/lookup/")
                .route(web::get().to(lookup_collections_containing_article)),
        )
        .service(
            web::resource("/collections/{id}/")
                .route(web::get().to(get_collection_by_id))
                .route(web::post().to(modify_collection))
                .route(web::delete().to(delete_collection)),
        )
        .service(
            web::resource("/collections/{id}/items/")
                .route(web::post().to(add_collection_item_to_collection)),
        )
        .service(
            web::resource("/collections/{collection_id}/items/{item_id}/")
                .route(web::get().to(get_collection_item_in_collection_by_id))
                .route(web::post().to(modify_collection_item_in_collection))
                .route(web::delete().to(remove_collection_item_from_collection)),
        )
}
