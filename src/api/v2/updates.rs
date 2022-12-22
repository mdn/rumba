use std::collections::HashMap;
use std::str::FromStr;

use crate::db::types::BcdUpdateEventType;
use crate::db::v2::bcd_updates::get_bcd_updates_paginated;
use crate::db::v2::model::{Event, Status};
use crate::{api::error::ApiError, db::Pool};
use actix_identity::Identity;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDate;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

fn array_like<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(res) = s {
        let collected: Vec<String> = res
            .split(',')
            .map(|val| String::from_str(val).unwrap())
            .collect();
        return Ok(Some(collected));
    }
    Ok(None)
}
#[derive(Debug, Serialize, Deserialize)]
pub enum AscOrDesc {
    #[serde(alias = "asc")]
    Asc,
    #[serde(alias = "desc")]
    Desc,
}

#[derive(Deserialize, Serialize)]
pub struct BcdUpdatesQueryParams {
    #[serde(default, deserialize_with = "array_like")]
    pub browsers: Option<Vec<String>>,
    #[serde(default, deserialize_with = "array_like")]
    pub category: Option<Vec<String>>,
    pub page: Option<i64>,
    pub q: Option<String>,
    pub show: Option<String>,
    pub sort: Option<AscOrDesc>,
}

#[derive(Serialize, Hash, Eq, PartialEq)]
pub enum UpdateType {
    #[serde(rename(serialize = "browser_grouping"))]
    BrowserGrouping,
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
    pub release_date: NaiveDate,
}

fn query_contains_restricted_filters(query: &BcdUpdatesQueryParams) -> bool {
    query.browsers.is_some()
        || query.q.is_some()
        || query.sort.is_some()
        || query.category.is_some()
        || query.show.is_some()
}
pub async fn get_updates_watched(
    _req: HttpRequest,
    pool: web::Data<Pool>,
    user_id: Option<Identity>,
    mut query: web::Query<BcdUpdatesQueryParams>,
) -> Result<HttpResponse, ApiError> {
    query.show = Some("watched".to_string());
    get_updates(_req, pool, user_id, query).await
}

pub async fn get_updates(
    _req: HttpRequest,
    pool: web::Data<Pool>,
    user_id: Option<Identity>,
    query: web::Query<BcdUpdatesQueryParams>,
) -> Result<HttpResponse, ApiError> {
    if user_id.is_none() && query_contains_restricted_filters(&query) {
        return Err(ApiError::LoginRequiredForFeature("BCD Filters".to_string()));
    }

    let mut conn_pool = pool.get()?;
    let updates = get_bcd_updates_paginated(&mut conn_pool, &query, user_id)?;
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
                release_date: key.4,
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
