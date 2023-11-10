//! Custom serialization of DateTime to conform with the JWT spec (RFC 7519 section 2, "Numeric Date")

use serde::{self, Deserialize, Deserializer, Serializer};
use time::OffsetDateTime;

/// Serializes an OffsetDateTime to a Unix timestamp (seconds since 1970/1/1T00:00:00T)
pub fn serialize<S>(date: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let timestamp = date.unix_timestamp();
    serializer.serialize_i64(timestamp)
}

/// Attempts to deserialize an i64 and use as a Unix timestamp
pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    OffsetDateTime::from_unix_timestamp(i64::deserialize(deserializer)?)
        .map_err(|_| serde::de::Error::custom("invalid Unix timestamp value"))
}
