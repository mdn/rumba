use serde::{Deserialize, Serialize};

use crate::{api::error::ApiError, settings::HARSH};

#[derive(Serialize, Deserialize)]
pub struct EncodedId {
    pub id: String,
}

impl EncodedId {
    pub fn get(&self) -> Result<i64, ApiError> {
        let val = HARSH.decode(&self.id).map_err(|_| ApiError::MalformedUrl)?;
        Ok(val[0] as i64)
    }

    pub fn encode(val: i64) -> String {
        HARSH.encode(&[val as u64])
    }

    pub fn decode<T: AsRef<str>>(val: T) -> Result<i64, ApiError> {
        let val = HARSH.decode(val).map_err(|_| ApiError::MalformedUrl)?;

        Ok(val[0] as i64)
    }
}
