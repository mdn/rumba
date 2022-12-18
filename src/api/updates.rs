use std::collections::HashMap;
use std::str::FromStr;
use std::{fs::File, io::BufReader};

use crate::db::schema::*;
use crate::db::types::BcdUpdateEventType;
use crate::db::Pool;
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use actix_web::{
    web::{self, Data},
    HttpRequest, HttpResponse,
};
use chrono::NaiveDate;
use diesel::PgConnection;
use reqwest::Client;

use super::error::ApiError;
use crate::diesel::BoolExpressionMethods;

pub async fn update_bcd(
    pool: Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;

    synchronize_browers_and_releases(&mut conn).await?;
    synchronize_features(&mut conn).await?;
    synchronize_updates(&mut conn).await?;

    Ok(HttpResponse::Accepted().finish())
}

async fn synchronize_browers_and_releases(pool: &mut PgConnection) -> Result<(), ApiError> {
    let file = File::open("browsers.json").map_err(|_| ApiError::Unknown)?;
    let reader = BufReader::new(file);
    let json: serde_json::Value =
        serde_json::from_reader(reader).expect("Error reading browsers.json");

    let mut browser_values = Vec::new();
    let mut releases = Vec::new();
    json.as_object().unwrap().iter().for_each(|(k, v)| {
        browser_values.push((
            browsers::name.eq(k.as_str()),
            browsers::accepts_flags.eq(v["accepts_flags"].as_bool().unwrap()),
            browsers::accepts_webextensions.eq(v["accepts_webextensions"].as_bool().unwrap()),
            browsers::pref_url.eq(v["pref_url"].as_str()),
            browsers::preview_name.eq(v["preview_name"].as_str()),
        ));

        for (release, value) in v["releases"].as_object().unwrap() {
            match value["engine"].as_str() {
                Some(_) => (),
                None => error!("No engine for {:?}", value),
            }
            match value["release_date"].as_str() {
                Some(_) => (),
                None => error!("No release_date for {:?}", value),
            }
            let _release_date: Option<NaiveDate> = value["release_date"]
                .as_str()
                .map_or_else(|| None, |v| Some(NaiveDate::from_str(v).unwrap()));
            if _release_date.is_none() {
                return;
            }
            releases.push((
                browser_releases::browser.eq(k.as_str()),
                browser_releases::engine.eq(value["engine"].as_str().unwrap_or("Unknown")),
                browser_releases::engine_version
                    .eq(value["engine_version"].as_str().unwrap_or("Unknown")),
                browser_releases::release_id.eq(release),
                browser_releases::release_date.eq(_release_date.unwrap()),
                browser_releases::release_notes.eq(value["release_notes"].as_str()),
                browser_releases::status.eq(value["status"].as_str()),
            ));
        }
    });

    diesel::insert_into(crate::db::schema::browsers::table)
        .values(browser_values)
        .execute(pool)
        .map_err(|e| error!("{:?}", e));

    diesel::insert_into(crate::db::schema::browser_releases::table)
        .values(releases)
        .execute(pool)
        .map_err(|e| error!("{:?}", e));

    Ok(())
}

async fn synchronize_features(pool: &mut PgConnection) -> Result<(), ApiError> {
    let file = File::open("features_3.json").map_err(|_| ApiError::Unknown)?;
    let reader = BufReader::new(file);
    let json: serde_json::Value =
        serde_json::from_reader(reader).expect("Error reading features.json");

    let mut features = Vec::new();
    json.as_array().unwrap().iter().for_each(|val| {
        if val["source_file"].as_str().is_none() {
            error!("No source file found for path. Much confuse {:?}", val);
            return;
        }
        features.push((
            features::path.eq(val["path"].as_str().unwrap()),
            features::mdn_url.eq(val["mdn_url"].as_str()),
            features::source_file.eq(val["source_file"].as_str().unwrap()),
            features::spec_url.eq(val["spec_url"].as_str()),
            features::deprecated.eq(val["status"]
                .as_object()
                .map_or(None, |v| v["deprecated"].as_bool())),
            features::experimental.eq(val["status"]
                .as_object()
                .map_or(None, |v| v["experimental"].as_bool())),
            features::standard_track.eq(val["status"]
                .as_object()
                .map_or(None, |v| v["standard_track"].as_bool())),
        ));
    });

    while features.len() > 0 {
        let mut batch_size = 1000;
        if batch_size > features.len() {
            batch_size = features.len();
        }
        let drained: Vec<_> = features.drain(0..batch_size).collect();
        diesel::insert_into(features::table)
            .values(drained)
            .execute(pool)
            .map_err(|e| error!("{:?}", e));
    }

    Ok(())
}

async fn synchronize_updates(pool: &mut PgConnection) -> Result<(), ApiError> {
    let file = File::open("added_removed.json").map_err(|_| ApiError::Unknown)?;
    let reader = BufReader::new(file);
    let json: serde_json::Value =
        serde_json::from_reader(reader).expect("Error reading added_removed.json");

    let mut release_versions_cached = HashMap::<String, i64>::new();
    let mut feature_info_cached = HashMap::<String, i64>::new();

    json.as_array().unwrap().iter().for_each(|val| {
        let _browser = val
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["browser"].as_str().is_some())
            .unwrap();
        let key = format!(
            "{:1}-{:2}",
            _browser["browser"].as_str().unwrap(),
            _browser["version"].as_str().unwrap()
        );
        let added = val
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["added"].as_array().is_some() && x["added"].as_array().unwrap().len() > 0)
            .map(|v| &v["added"]);
        let removed = val
            .as_array()
            .unwrap()
            .iter()
            .find(|x| {
                x["removed"].as_array().is_some() && x["removed"].as_array().unwrap().len() > 0
            })
            .map(|v| &v["removed"]);

        let mut _added_ = Vec::new();
        let mut _removed_ = Vec::new();
        if added.is_none() && removed.is_none() {
            return;
        }
        let mut _browser_release_id = match release_versions_cached.get(&key) {
            Some(val) => val.clone(),
            None => {
                let _release_id: Result<i64, diesel::result::Error> = browser_releases::table
                    .select(browser_releases::id)
                    .filter(
                        browser_releases::browser
                            .eq(_browser["browser"].as_str().unwrap())
                            .and(
                                browser_releases::release_id
                                    .eq(_browser["version"].as_str().unwrap()),
                            ),
                    )
                    .first(pool);
                if _release_id.is_err() {
                    error!(
                        "{:}, {:}",
                        _browser["browser"].as_str().unwrap(),
                        _browser["version"].as_str().unwrap()
                    );
                }
                let unwrapped = _release_id.unwrap();

                release_versions_cached.insert(key, unwrapped.clone());
                unwrapped.clone()
            }
        };

        if let Some(val) = added {
            for path in val.as_array().unwrap() {
                let feature_id = match feature_info_cached.get(path.as_str().unwrap()) {
                    Some(val) => val.clone(),
                    None => {
                        let _feature_id: Result<i64, diesel::result::Error> = features::table
                            .select(features::id)
                            .filter(features::path.eq(path.as_str().unwrap()))
                            .first(pool);
                        if _feature_id.is_err() {
                            error!("Error {:}", path.as_str().unwrap());
                        }
                        let unwrapped = _feature_id.unwrap();

                        feature_info_cached
                            .insert(path.as_str().unwrap().to_owned(), unwrapped.clone());
                        unwrapped.clone()
                    }
                };
                _added_.push((
                    bcd_updates::browser_release.eq(_browser_release_id),
                    bcd_updates::document_id.eq(1),
                    bcd_updates::event_type.eq(BcdUpdateEventType::AddedStable),
                    bcd_updates::feature.eq(feature_id),
                ));
            }
        }

        if let Some(val) = removed {
            for path in val.as_array().unwrap() {
                let feature_id = match feature_info_cached.get(path.as_str().unwrap()) {
                    Some(val) => val.clone(),
                    None => {
                        let _feature_id: Result<i64, diesel::result::Error> = features::table
                            .select(features::id)
                            .filter(features::path.eq(path.as_str().unwrap()))
                            .first(pool);

                        let unwrapped = _feature_id.unwrap();

                        feature_info_cached
                            .insert(path.as_str().unwrap().to_owned(), unwrapped.clone());
                        unwrapped.clone()
                    }
                };
                _removed_.push((
                    bcd_updates::browser_release.eq(_browser_release_id),
                    bcd_updates::document_id.eq(1),
                    bcd_updates::event_type.eq(BcdUpdateEventType::RemovedStable),
                    bcd_updates::feature.eq(feature_id),
                ));
            }
        }

        diesel::insert_into(bcd_updates::table)
            .values(_added_)
            .execute(pool)
            .map_err(|e| error!("{:?} \n {:?}, {:?}", e, added, _browser_release_id));

        diesel::insert_into(bcd_updates::table)
            .values(_removed_)
            .execute(pool)
            .map_err(|e| error!("{:?} \n {:?} , {:?}", e, removed, _browser_release_id));
    });
    Ok(())
}
