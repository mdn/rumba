use std::collections::HashMap;

use actix_web::{HttpRequest, web, HttpResponse};
use chrono::{NaiveDateTime, Utc, Datelike};
use serde::{Serialize, Deserialize};
use serde_json::{Map, Value};
use map_macro::map;
use crate::{db::{Pool}, api::error::ApiError};
use crate::helpers::to_utc;

#[derive(Deserialize)]
pub struct BcdUpdatesQueryParams {
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Serialize, Hash, Eq, PartialEq)]
pub enum UpdateType { 
    BrowserGrouping,
    AddedMissing,
    SubfeatureAdded
}

pub type UpdateMap = HashMap<UpdateType,BcdUpdate>;

#[derive(Serialize)]
pub struct BcdUpdatesPaginatedResponse {
    pub updates: Vec<UpdateMap>,
    pub query: String,
}

#[derive(Serialize, Clone)]
pub struct BcdUpdateEvent {
    #[serde(rename(serialize = "type"))]
    pub _type: String,    
    pub label: String,
    pub deprecated: Option<bool>,
    pub preview: Option<bool>,
    pub supported_engines: Option<i16>,
    pub mdn_url: Option<String>,
    pub bcd_path: String
}

#[derive(Serialize)]
pub struct BrowserInfo {
    pub display_name: String, 
    pub version: i8,
}

#[derive(Serialize)]
pub struct BcdUpdate {
    pub browser: Option<BrowserInfo>, 
    pub events: Vec<BcdUpdateEvent>,
    #[serde(serialize_with = "to_utc")]
    pub published_at: NaiveDateTime,
}

pub async fn get_updates(
    _req: HttpRequest,    
    pool: web::Data<Pool>,
    query: web::Query<BcdUpdatesQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    // get_bcd_updates_paginated(&mut conn_pool,&query.into_inner()).await;

let mut mockUpdates = vec![
    BcdUpdateEvent { 
    _type: "support_added".to_string(), 
    label: ":modal".to_string(), 
    deprecated: None, 
    preview: None, 
    supported_engines: Some(2), 
    mdn_url: Some("https://developer.allizom.org/en-US/docs/Web/CSS/:modal".to_string()), 
    bcd_path: "css.selectors.modal".to_string()
},
BcdUpdateEvent { 
    _type: "support_added".to_string(), 
    label: "translate()".to_string(), 
    deprecated: None, 
    preview: None, 
    supported_engines: Some(1), 
    mdn_url: Some("https://developer.allizom.org/en-US/docs/Web/CSS/transform-function/translate".to_string()), 
    bcd_path: "css.types.transform-function.translate".to_string()
},
BcdUpdateEvent { 
    _type: "preview_added".to_string(), 
    label: "MIDIInput".to_string(), 
    deprecated: None, 
    preview: Some(true), 
    supported_engines: None, 
    mdn_url: Some("https://developer.allizom.org/en-US/docs/Web/API/MIDIInput".to_string()), 
    bcd_path: "api.MIDIInput".to_string()
}];

    let update_one = (UpdateType::BrowserGrouping, BcdUpdate {
        browser: Some(BrowserInfo { display_name: "Opera".to_string(), version: 91 }),
        published_at:  Utc::now().naive_utc(),
        events: mockUpdates.iter_mut().map(|all| all.to_owned()).collect(),
    });
    let update_two = (UpdateType::BrowserGrouping, BcdUpdate {
        browser: Some(BrowserInfo { display_name: "Firefox".to_string(), version: 93 }),
        published_at:  Utc::now().naive_utc().with_month(10).unwrap(),
        events: mockUpdates,
    });
    let update_three = (UpdateType::SubfeatureAdded, BcdUpdate {
        browser: None,
        events: vec![BcdUpdateEvent { 
            _type: "Subfeature Added".to_string(), 
            label: "translate()".to_string(), 
            deprecated: None, 
            preview: None, 
            supported_engines: None, 
            mdn_url: None, 
            bcd_path: "some.path".to_string() }],
        published_at: Utc::now().naive_utc().with_month(9).unwrap(),
    });
    
    let response = BcdUpdatesPaginatedResponse {
        updates: vec![map! {update_one.0 => update_one.1} ,    
        map! {update_two.0 => update_two.1},
        map! {update_three.0 => update_three.1}],
        query: "We'll pass back the query context here for filters".to_string()
    };
    Ok(HttpResponse::Ok().json(response))
}