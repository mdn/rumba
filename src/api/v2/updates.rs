use std::collections::HashMap;

use crate::db::types::BcdUpdateEventType;
use crate::db::v2::bcd_updates::get_bcd_updates_paginated;
use crate::db::v2::model::{BcdUpdateQuery, Event, Status};
use crate::{api::error::ApiError, db::Pool};
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{NaiveDate};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct BcdUpdatesQueryParams {
    pub q: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Serialize, Hash, Eq, PartialEq)]
pub enum UpdateType {
    #[serde(rename(serialize = "browser_grouping"))]
    BrowserGrouping,
    #[serde(rename(serialize = "added_missing"))]
    AddedMissing,
    #[serde(rename(serialize = "added_subfeatures"))]
    SubfeatureAdded,
}

pub type UpdateMap = HashMap<UpdateType, BcdUpdate>;

#[derive(Serialize)]
pub struct BcdUpdatesPaginatedResponse {
    pub data: Vec<BcdUpdate>,
    pub query: String,
    pub last: u32,
}

#[derive(Serialize)]
pub struct BcdUpdateEvent {
    pub added: Vec<FeatureInfo>,
    pub removed: Vec<FeatureInfo>,
}

#[derive(Serialize)]
pub struct FeatureInfo {
    pub path: String,
    pub compat: CompatInfo,
}

#[derive(Serialize)]
pub struct StatusInfo {
    deprecated: bool,
    experimental: bool,
    standard_track: bool,
}

#[derive(Serialize)]
pub struct CompatInfo {
    pub mdn_url: Option<String>,
    pub source_file: Option<String>,
    pub spec_url: Option<String>,
    pub status: Option<StatusInfo>,
    pub engines: Vec<String>,
}
#[derive(Serialize)]
pub struct BrowserInfo {
    pub browser: String,
    pub version: String,
    pub name: String,
    pub engine: String,
    pub engine_version: String,
    pub release_notes: String,
}

#[derive(Serialize)]
pub struct BcdUpdate {
    #[serde(rename(serialize = "type"))]
    pub _type: UpdateType,
    #[serde(flatten)]
    pub browser: Option<BrowserInfo>,
    pub events: BcdUpdateEvent,
    pub date: NaiveDate,
}

pub async fn get_updates(
    _req: HttpRequest,
    pool: web::Data<Pool>,
    query: web::Query<BcdUpdatesQueryParams>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let updates = get_bcd_updates_paginated(&mut conn_pool, &query.into_inner())?;
    let mapped_updates = updates
        .into_iter()
        .group_by(|key| {
            (
                key.browser.clone(),
                key.release_id.clone(),
                key.engine.clone(),
                key.engine_version.clone(),
                key.release_date.clone(),
            )
        })
        .into_iter()
        .map(|(key, group)| {
            let collected = group.collect::<Vec<BcdUpdateQuery>>();
            BcdUpdate {
                _type: UpdateType::BrowserGrouping,
                browser: Some(BrowserInfo {
                    version: key.1,
                    name: key.0.to_string(),
                    browser: key.0.to_string(),
                    engine_version: key.3,
                    engine: key.2,
                    release_notes: "".to_string(),
                }),
                date: key.4,
                events: BcdUpdateEvent {
                    added: collected
                        .iter()
                        .map(|val| {
                            val.compat
                                .events
                                .iter()
                                .filter(|to_filter| {
                                    to_filter.event_type.eq(&BcdUpdateEventType::AddedStable)
                                })
                                .map(|hello| hello.into())
                        })
                        .flatten()
                        .collect(),
                    removed: collected
                        .iter()
                        .map(|val| {
                            val.compat.events
                                .iter()
                                .filter(|to_filter| {
                                    to_filter.event_type.eq(&BcdUpdateEventType::RemovedStable)
                                })
                                .map(|hello| hello.into())
                        })
                        .flatten()
                        .collect(),
                },
            }
        })
        .collect();
    let response = BcdUpdatesPaginatedResponse {
        data: mapped_updates,
        query: "We'll pass back the query context here for filters".to_string(),
        last: 40,
    };
    Ok(HttpResponse::Ok().json(response))
}

impl From<&Event> for FeatureInfo {
    fn from(val: &Event) -> Self {
        FeatureInfo {
            path: val.path.clone(),
            compat: CompatInfo {
                mdn_url: val.mdn_url.clone(),
                source_file: val.source_file.clone(),
                spec_url: val.spec_url.clone(),
                status: val
                    .status
                    .as_ref()
                    .map_or(Option::<StatusInfo>::None, |val| {
                        Some(Into::<StatusInfo>::into(&*val))
                    }),
                engines: vec![],
            },
        }
    }
}

impl From<&Status> for StatusInfo {
    fn from(val: &Status) -> Self {
        StatusInfo {
            deprecated: val.deprecated,
            experimental: val.experimental,
            standard_track: val.standard_track,
        }
    }
}
