use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{from_value, Value};

pub fn deserialize_string_or_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: Value = Deserialize::deserialize(deserializer)?;
    match from_value::<Vec<T>>(value.clone()) {
        Ok(val) => Ok(val),
        Err(_) => {
            let single_val: T = from_value(value).map_err(Error::custom)?;
            Ok(vec![single_val])
        }
    }
}

pub mod serde_utc_seconds_f {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds: serde_json::Number = Deserialize::deserialize(deserializer)?;
        let (secs, nsecs) = if let Some(secs) = seconds.as_i64() {
            (secs, 0u32)
        } else {
            let secs_f64 = seconds.as_f64().ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "failed to parse `{}` as UTC datetime (in seconds)",
                    seconds
                ))
            })?;
            let secs = secs_f64.floor();
            (
                secs as i64,
                ((secs_f64 - secs) * 1_000_000_000.).floor() as u32,
            )
        };
        Utc.timestamp_opt(secs, nsecs).single().ok_or_else(|| {
            serde::de::Error::custom(format!(
                "failed to parse `{}` as UTC datetime (in seconds)",
                seconds
            ))
        })
    }

    #[allow(dead_code)]
    pub fn serialize<S>(v: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        v.timestamp().serialize(serializer)
    }
}

pub mod serde_utc_milliseconds {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds: serde_json::Number = Deserialize::deserialize(deserializer)?;
        let (secs, nsecs) = if let Some(secs) = seconds.as_i64() {
            (secs / 1000, ((secs % 1000) * 1_000_000) as u32)
        } else {
            return Err(serde::de::Error::custom(format!(
                "failed to parse `{}` as UTC datetime (in milliseconds)",
                seconds
            )));
        };
        Utc.timestamp_opt(secs, nsecs).single().ok_or_else(|| {
            serde::de::Error::custom(format!(
                "failed to parse `{}` as UTC datetime (in milliseconds)",
                seconds
            ))
        })
    }

    pub fn serialize<S>(v: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (v.timestamp() * 1000).serialize(serializer)
    }
}
