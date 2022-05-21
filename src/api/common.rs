use serde::Deserialize;

#[derive(Deserialize)]
pub enum Sorting {
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "created")]
    Created,
}
