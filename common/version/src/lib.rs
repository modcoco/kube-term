pub use anyhow;
pub use axum;
pub use chrono;
pub use dotenv;
pub use reqwest;
pub use sqlx;
pub use tokio;

pub mod constants;
use constants::*;

#[derive(Debug)]
pub struct PodSecrets {
    pub cacrt: Vec<u8>,
    pub namespace: String,
    pub token: String,
}

impl Default for PodSecrets {
    fn default() -> Self {
        Self::new()
    }
}

impl PodSecrets {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        let mut cacrt_path = CACRT_PATH;
        let mut namespace = NAMESPACE_PATH;
        let mut token_path = TOKEN_PATH;

        let local_cacrt_path = &std::env::var("CA_CERT_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", cacrt_path);
            String::default()
        });

        let local_namespace = &std::env::var("NAMESPACE_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", namespace);
            String::default()
        });

        let local_token_path = &std::env::var("TOKEN_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", token_path);
            String::default()
        });

        match std::env::var("APP_ENV") {
            Ok(app_env) => {
                if app_env == APP_ENV_LOCAL {
                    cacrt_path = local_cacrt_path;
                    namespace = local_namespace;
                    token_path = local_token_path;
                    println!("{}", token_path)
                }
            }
            Err(_) => {
                tracing::debug!("Use default kube config, {}", APP_ENV_PRODUCT)
            }
        }

        let cacrt = match std::fs::read(cacrt_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read CA certificate: {}", e);
                Vec::<u8>::new()
            }
        };
        let namespace = match std::fs::read_to_string(namespace) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read namespace: {}", e);
                String::new()
            }
        };
        let token = match std::fs::read_to_string(token_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read token: {}", e);
                String::new()
            }
        };

        Self {
            cacrt,
            namespace,
            token,
        }
    }
}

pub fn url_https_builder(domain: &str, port: &str, path: &str) -> String {
    [URL_HTTPS, domain, COLON, port, path].concat()
}

pub fn url_http_builder(domain: &str, port: &str, path: &str) -> String {
    [URL_HTTP, domain, COLON, port, path].concat()
}
