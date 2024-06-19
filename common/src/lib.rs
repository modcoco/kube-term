pub use anyhow;
pub use axum;
pub use chrono;
pub use reqwest;
pub use sqlx;
pub use tokio;

mod constants;
use constants::*;

pub struct PodSecrets {
    pub cacrt: String,
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
        let cacrt = match std::fs::read_to_string(CACRT_PATH) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read CA certificate: {}", e);
                String::new()
            }
        };
        let namespace = match std::fs::read_to_string(NAMESPACE_PATH) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read namespace: {}", e);
                String::new()
            }
        };
        let token = match std::fs::read_to_string(TOKEN_PATH) {
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
