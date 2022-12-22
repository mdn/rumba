use std::collections::HashMap;
use std::str::FromStr;
use std::{fs::File, io::BufReader};

use crate::api::error::ApiError;
use crate::db::schema::{self, *};
use crate::db::types::BcdUpdateEventType;
use crate::db::Pool;
use crate::diesel::ExpressionMethods;
use crate::diesel::QueryDsl;
use crate::diesel::RunQueryDsl;
use actix_http::StatusCode;
use actix_web::{web::Data, HttpResponse};
use chrono::NaiveDate;
use diesel::{update, PgConnection};
use reqwest::Client;
use serde_json::Value;

use crate::diesel::BoolExpressionMethods;

pub async fn update_bcd(pool: Data<Pool>, client: Data<Client>) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    info!("Synchronize browsers");
    synchronize_browers_and_releases(&mut conn).await?;
    info!("Synchronize features");
    synchronize_features(&mut conn).await?;
    info!("Synchronize paths + bcd mappings");
    synchronize_path_mappings(&mut conn, client).await?;
    synchronize_updates(&mut conn).await?;

    Ok(HttpResponse::Accepted().finish())
}

async fn synchronize_browers_and_releases(pool: &mut PgConnection) -> Result<(), ApiError> {
    let file = File::open("browsers.json")
        .map_err(|err| ApiError::Generic(format!("Error loading browsers.json: {:}", err)))?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).map_err(|err| {
        ApiError::Generic(format!(
            "Error deserializing data from browsers.json: {:}",
            err
        ))
    })?;

    let mut browser_values = Vec::new();
    let mut releases = Vec::new();
    json.as_object().unwrap().iter().for_each(|(k, v)| {
        browser_values.push((
            browsers::name.eq(k.as_str()),
            browsers::display_name.eq(v["name"].as_str().unwrap()),
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
        .on_conflict_do_nothing()
        .execute(pool)
        .map_err(|e| ApiError::Generic(e.to_string()))?;

    diesel::insert_into(crate::db::schema::browser_releases::table)
        .values(releases)
        .on_conflict_do_nothing()
        .execute(pool)
        .map_err(|e| ApiError::Generic(e.to_string()))?;
    Ok(())
}

async fn synchronize_features(pool: &mut PgConnection) -> Result<(), ApiError> {
    let file = File::open("features.json")
        .map_err(|err| ApiError::Generic(format!("Error loading features.json: {:}", err)))?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader)
        .map_err(|err| ApiError::Generic(format!("Error deserializing features.json: {:}", err)))?;
    let mut features = Vec::new();
    json.as_array().unwrap().iter().for_each(|val| {
        if val["source_file"].as_str().is_none() {
            error!("No source file found for path. {:?}", val);
            return;
        }
        features.push((
            bcd_features::path.eq(val["path"].as_str().unwrap()),
            bcd_features::mdn_url.eq(val["mdn_url"].as_str()),
            bcd_features::source_file.eq(val["source_file"].as_str().unwrap()),
            bcd_features::spec_url.eq(val["spec_url"].as_str()),
            bcd_features::deprecated.eq(val["status"]
                .as_object()
                .and_then(|v| v["deprecated"].as_bool())),
            bcd_features::experimental.eq(val["status"]
                .as_object()
                .and_then(|v| v["experimental"].as_bool())),
            bcd_features::standard_track.eq(val["status"]
                .as_object()
                .and_then(|v| v["standard_track"].as_bool())),
        ));
    });

    while !features.is_empty() {
        let mut batch_size = 1000;
        if batch_size > features.len() {
            batch_size = features.len();
        }
        let drained: Vec<_> = features.drain(0..batch_size).collect();
        let res = diesel::insert_into(bcd_features::table)
            .values(drained)
            .execute(pool)
            .map_err(|e| error!("{:?}", e));

        if let Err(val) = res {
            warn!("Error adding features {:?}", val);
        }
    }

    Ok(())
}

async fn synchronize_updates(pool: &mut PgConnection) -> Result<(), ApiError> {
    let file = File::open("added_removed.json")
        .map_err(|err| ApiError::Generic(format!("Error loading added_removed.json: {:}", err)))?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader)
        .map_err(|err| ApiError::Generic(format!("Error Deserializing features.json: {:}", err)))?;

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
            .find(|x| x["added"].as_array().is_some() && !x["added"].as_array().unwrap().is_empty())
            .map(|v| &v["added"]);
        let removed = val
            .as_array()
            .unwrap()
            .iter()
            .find(|x| {
                x["removed"].as_array().is_some() && !x["removed"].as_array().unwrap().is_empty()
            })
            .map(|v| &v["removed"]);

        let mut _added_ = Vec::new();
        let mut _removed_ = Vec::new();
        if added.is_none() && removed.is_none() {
            return;
        }
        let mut _browser_release_id = match release_versions_cached.get(&key) {
            Some(val) => *val,
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

                release_versions_cached.insert(key, unwrapped);
                unwrapped
            }
        };

        if let Some(val) = added {
            for path in val.as_array().unwrap() {
                let feature_id = match feature_info_cached.get(path.as_str().unwrap()) {
                    Some(val) => *val,
                    None => {
                        let _feature_id: Result<i64, diesel::result::Error> = bcd_features::table
                            .select(bcd_features::id)
                            .filter(bcd_features::path.eq(path.as_str().unwrap()))
                            .first(pool);
                        if _feature_id.is_err() {
                            error!("Error {:}", path.as_str().unwrap());
                        }
                        let unwrapped = _feature_id.unwrap();

                        feature_info_cached.insert(path.as_str().unwrap().to_owned(), unwrapped);
                        unwrapped
                    }
                };
                _added_.push((
                    bcd_updates::browser_release.eq(_browser_release_id),
                    bcd_updates::event_type.eq(BcdUpdateEventType::AddedStable),
                    bcd_updates::feature.eq(feature_id),
                ));
            }
        }

        if let Some(val) = removed {
            for path in val.as_array().unwrap() {
                let feature_id = match feature_info_cached.get(path.as_str().unwrap()) {
                    Some(val) => *val,
                    None => {
                        let _feature_id: Result<i64, diesel::result::Error> = bcd_features::table
                            .select(bcd_features::id)
                            .filter(bcd_features::path.eq(path.as_str().unwrap()))
                            .first(pool);

                        let unwrapped = _feature_id.unwrap();

                        feature_info_cached.insert(path.as_str().unwrap().to_owned(), unwrapped);
                        unwrapped
                    }
                };
                _removed_.push((
                    bcd_updates::browser_release.eq(_browser_release_id),
                    bcd_updates::event_type.eq(BcdUpdateEventType::RemovedStable),
                    bcd_updates::feature.eq(feature_id),
                ));
            }
        }

        let added_results = diesel::insert_into(bcd_updates::table)
            .values(_added_)
            .on_conflict_do_nothing()
            .execute(pool)
            .map_err(|e| {
                error!("{:?} \n {:?}, {:?}", e, added, _browser_release_id);
                e
            });
        if added_results.is_err() {
            warn!("Error creating added bcd updates")
        }
        let remove_results = diesel::insert_into(bcd_updates::table)
            .values(_removed_)
            .on_conflict_do_nothing()
            .execute(pool)
            .map_err(|e| {
                error!("{:?} \n {:?} , {:?}", e, removed, _browser_release_id);
                e
            });
        if remove_results.is_err() {
            warn!("Error creating feature removed bcd updates")
        }
    });
    Ok(())
}

async fn synchronize_path_mappings(
    pool: &mut PgConnection,
    client: Data<Client>,
) -> Result<(), ApiError> {
    let metadata_url = "https://developer.mozilla.org/en-US/metadata.json";
    let values = client.get(metadata_url.to_owned()).send().await.map_err(
        |err: reqwest::Error| match err.status() {
            Some(StatusCode::NOT_FOUND) => {
                warn!("Error NOT_FOUND fetching all metadata {} ", &metadata_url);
                ApiError::DocumentNotFound
            }
            _ => {
                warn!("Error Unknown fetching all metadata {} ", &metadata_url);
                ApiError::Unknown
            }
        },
    )?;

    let json: Value = values
        .json()
        .await
        .map_err(|_| ApiError::DocumentNotFound)?;

    struct PathAndShortTitle {
        path: String,
        mdn_url: String,
        short_title: String,
    }

    //1. Get all values with a bcd path, extract path, mdn_url, short title.
    let mut path_map: HashMap<String, (String, String)> = HashMap::new();

    let extract: Vec<PathAndShortTitle> = json
        .as_array()
        .unwrap()
        .iter()
        .filter(|val| val["browserCompat"].as_array().is_some())
        .map(|filtered| {
            let paths = filtered["browserCompat"].as_array().unwrap();
            if paths.len() > 1 {
                warn!("Multiple paths detected for {:?}", paths);
            }

            for path in paths {
                path_map.insert(
                    path.as_str().unwrap().try_into().unwrap(),
                    (
                        String::from_str(filtered["mdn_url"].as_str().unwrap()).unwrap(),
                        String::from_str(filtered["short_title"].as_str().unwrap()).unwrap(),
                    ),
                );
            }

            PathAndShortTitle {
                path: String::from_str(paths[0].as_str().unwrap()).unwrap(),
                mdn_url: String::from_str(filtered["mdn_url"].as_str().unwrap()).unwrap(),
                short_title: String::from_str(filtered["short_title"].as_str().unwrap()).unwrap(),
            }
        })
        .collect();

    extract.iter().for_each(|path_and_title| {
        let res =
            update(schema::bcd_features::table.filter(bcd_features::path.eq(&path_and_title.path)))
                .set((
                    bcd_features::mdn_url.eq(&path_and_title.mdn_url),
                    bcd_features::short_title.eq(&path_and_title.short_title),
                ))
                .execute(pool);
        if let Err(err) = res {
            warn!("Error updating {:}, {:?}", &path_and_title.path, err);
        }
    });

    //2. Find paths with missing info and patch them to the next higher subpath.
    let null_vals: Vec<String> = schema::bcd_features::table
        .select(schema::bcd_features::path)
        .filter(bcd_features::mdn_url.is_null())
        .get_results::<String>(pool)?;
    //Let's find all the features without a
    for val in null_vals {
        let mut parts: Vec<&str> = val.split('.').collect();
        parts.pop();
        while !parts.is_empty() {
            let subpath = parts.join(".");
            info!("checking subpath {:} for {:}", subpath, val);
            if let Some(replacement) = path_map.get(&subpath) {
                info!(
                    "Replacing missing url + title for path {:} with {:}'s ({:},{:})",
                    val, subpath, &replacement.0, &replacement.1
                );
                let res =
                    update(schema::bcd_features::table.filter(schema::bcd_features::path.eq(&val)))
                        .set((
                            schema::bcd_features::mdn_url.eq(&replacement.0),
                            schema::bcd_features::short_title.eq(&replacement.1),
                        ))
                        .execute(pool);
                if let Err(err) = res {
                    warn!(
                        "Error updating {:} with metadata from {:}, {:?}",
                        val, subpath, err
                    );
                }
                break;
            }
            parts.pop();
        }
    }

    Ok(())
}
