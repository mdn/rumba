use std::{path::Path, fs};

use anyhow::Error;
use once_cell::sync::Lazy;

const PROMPTS_YAML: & str = include_str!("../data/prompts.yaml");


pub fn read<'a>(path: Option<impl AsRef<Path>>) -> Result<Vec<Vec<String>>, Error> {
    if let Some(path) = path {
        Ok(serde_yaml::from_reader(fs::File::open(path)?)?)
    } else {
        Ok(serde_yaml::from_str(PROMPTS_YAML)?)
    }

}