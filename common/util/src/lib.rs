#![deny(unused_crate_dependencies)]
use common::chrono::{self, NaiveDateTime, TimeZone as _};
use common::serde::{self, Deserialize, Deserializer, Serializer};
use std::{fmt::Display, str::FromStr};

pub mod constants;
pub mod err;
pub mod rsp;

use chrono::{DateTime, FixedOffset, Utc};
use constants::*;

pub fn url_https_builder(domain: &str, port: &str, path: Option<&str>) -> String {
    base_http_builder(URL_HTTPS, domain, port, path)
}

pub fn url_http_builder(domain: &str, port: &str, path: Option<&str>) -> String {
    base_http_builder(URL_HTTP, domain, port, path)
}

fn base_http_builder(http_header: &str, domain: &str, port: &str, path: Option<&str>) -> String {
    match path {
        Some(p) => [http_header, domain, COLON, port, p].concat(),
        None => [http_header, domain, COLON, port].concat(),
    }
}

pub fn naive_datetime_with_offset(start_time: NaiveDateTime, offset_hours: i32) -> NaiveDateTime {
    let start_time_utc: DateTime<Utc> = DateTime::<Utc>::from_naive_utc_and_offset(start_time, Utc);
    let offset = FixedOffset::east_opt(offset_hours * 3600).expect("Failed to create offset");
    let start_time_with_offset = start_time_utc.with_timezone(&offset);

    start_time_with_offset.naive_local()
}

// 通用的从字符串反序列化为类型 T
pub fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<T>().map_err(serde::de::Error::custom)
}

// 特定类型的反序列化函数
pub fn deserialize_u64_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_from_str(deserializer)
}

pub fn deserialize_i64_from_str<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_from_str(deserializer)
}

// 通用的序列化类型 T 成字符串
pub fn serialize_to_str<S, T>(x: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: ToString,
{
    serializer.serialize_str(&x.to_string())
}

// 特定类型的序列化函数
pub fn serialize_u64_to_string<S>(x: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serialize_to_str(x, serializer)
}

pub fn serialize_i64_to_string<S>(x: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serialize_to_str(x, serializer)
}

pub fn serialize_i16_to_string<S>(x: &i16, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serialize_to_str(x, serializer)
}

// 日期时间类型的特定序列化函数
pub fn serialize_datetime_to_ymd_hms<S>(x: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let dt: chrono::DateTime<chrono::Local> = chrono::Local.from_utc_datetime(x);
    serializer.serialize_str(&dt.format("%F %T").to_string())
}

pub fn serialize_datetime_to_local_string<S>(
    x: &chrono::DateTime<chrono::Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let local_time = x.with_timezone(&chrono::Local);
    let formatted_dt = local_time.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&formatted_dt)
}

pub fn i64_round(value: i64) -> i64 {
    ((value as f64) / (1000.0 * 1000.0)).round() as i64
}
