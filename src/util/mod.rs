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

