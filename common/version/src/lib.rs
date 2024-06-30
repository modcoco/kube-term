pub use anyhow;
pub use axum;
pub use base64;
pub use chrono;
pub use dotenv;
pub use futures_util;
pub use native_tls;
pub use reqwest;
pub use rustls_pemfile;
pub use serde;
pub use sqlx;
pub use tokio;
pub use tokio_tungstenite;
pub use tracing;

pub mod constants;
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
