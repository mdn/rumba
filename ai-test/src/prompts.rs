use std::{fs, path::Path};

use anyhow::Error;

const PROMPTS_YAML: &str = include_str!("../data/prompts.yaml");

pub fn read(path: Option<impl AsRef<Path>>) -> Result<Vec<Vec<String>>, Error> {
    if let Some(path) = path {
        Ok(serde_yaml::from_reader(fs::File::open(path)?)?)
    } else {
        Ok(serde_yaml::from_str(PROMPTS_YAML)?)
    }
}
