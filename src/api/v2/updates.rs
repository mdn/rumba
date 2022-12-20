use std::collections::HashMap;
use std::str::FromStr;

use crate::db::types::BcdUpdateEventType;
use crate::db::v2::bcd_updates::get_bcd_updates_paginated;
use crate::db::v2::model::{Event, Status};
use crate::{api::error::ApiError, db::Pool};
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDate;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

fn array_like<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<&str> = Option::deserialize(deserializer)?;
    if let Some(res) = s {
        let collected: Vec<String> = res
            .split(',')
            .map(|val| String::from_str(val).unwrap())
            .collect();
        return Ok(Some(collected));
    }
    Ok(None)
}

#[derive(Deserialize, Serialize)]
pub struct BcdUpdatesQueryParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub live_since: Option<NaiveDate>,
    #[serde(default)]
    #[serde(deserialize_with = "array_like")]
    pub browsers: Option<Vec<String>>,
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
    pub query: BcdUpdatesQueryParams,
    pub last: i64,
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
    let updates = get_bcd_updates_paginated(&mut conn_pool, &query)?;
    let mapped_updates = updates
        .0
        .into_iter()
        .group_by(|key| {
            (
                key.browser.clone(),
                key.engine_version.clone(),
                key.engine.clone(),
                key.name.clone(),
                key.release_date,
                key.release_id.clone(),
            )
        })
        .into_iter()
        .map(|(key, group)| {
            let collected = group.collect::<Vec<crate::db::v2::model::BcdUpdate>>();
            BcdUpdate {
                _type: UpdateType::BrowserGrouping,
                browser: Some(BrowserInfo {
                    browser: key.0.to_string(),
                    engine_version: key.1,
                    engine: key.2,
                    name: key.3.to_string(),
                    release_notes: "".to_string(),
                    version: key.5,
                }),
                date: key.4,
                events: BcdUpdateEvent {
                    added: collected
                        .iter()
                        .flat_map(|val| {
                            val.compat
                                .iter()
                                .filter(|to_filter| {
                                    to_filter.event_type.eq(&BcdUpdateEventType::AddedStable)
                                })
                                .map(|hello| hello.into())
                        })
                        .collect(),
                    removed: collected
                        .iter()
                        .flat_map(|val| {
                            val.compat
                                .iter()
                                .filter(|to_filter| {
                                    to_filter.event_type.eq(&BcdUpdateEventType::RemovedStable)
                                })
                                .map(|hello| hello.into())
                        })
                        .collect(),
                },
            }
        })
        .collect();
    let response = BcdUpdatesPaginatedResponse {
        data: mapped_updates,
        query: query.into_inner(),
        last: updates.1,
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
                status: val.status.as_ref().map(Into::<StatusInfo>::into),
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