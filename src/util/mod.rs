use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize, Debug)]
pub struct BrowserEntry {
    pub name: String,
    pub preview_name: Option<String>,
    pub browser_group: String,
}

pub fn normalize_uri(input: &str) -> String {
    input.to_lowercase().trim().to_string()
}

pub static BROWSERS: Lazy<HashMap<String, BrowserEntry>> = Lazy::new(|| {
    //Panic of not avaialble.
    let file = File::open("static/browsers.json").unwrap();
    let reader = BufReader::new(file);
    let map = serde_json::from_reader(reader);
    match map {
        Ok(browsers) => browsers,
        Err(err) => {
            panic!("{:?}", err);
        }
    }
});
