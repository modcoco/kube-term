use common::{
    anyhow,
    constants::{APP_ENV_LOCAL, APP_ENV_PRODUCT, CACRT_PATH, NAMESPACE_PATH, TOKEN_PATH},
    dotenv,
    native_tls::{self, TlsConnector},
    tracing,
};

#[derive(Debug)]
pub struct ServiceAccountToken {
    pub kube_host: String,
    pub kube_port: String,
    pub cacrt: Vec<u8>,
    pub namespace: String,
    pub token: String,
}

impl Default for ServiceAccountToken {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceAccountToken {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        // Default Kube Env
        let mut cacrt_path = CACRT_PATH;
        let mut namespace = NAMESPACE_PATH;
        let mut token_path = TOKEN_PATH;
        let mut kube_host =
            &std::env::var("KUBERNETES_PORT_443_TCP_ADDR").unwrap_or_else(|_| String::default());
        let mut kube_port =
            &std::env::var("KUBERNETES_SERVICE_PORT").unwrap_or_else(|_| String::default());

        // Local Env
        let local_cacrt_path = &std::env::var("KUBERNETES_CA_CERT_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", cacrt_path);
            String::default()
        });

        let local_namespace = &std::env::var("KUBERNETES_NAMESPACE_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", namespace);
            String::default()
        });

        let local_token_path = &std::env::var("KUBERNETES_TOKEN_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", token_path);
            String::default()
        });

        let local_kube_host =
            &std::env::var("KUBERNETES_SERVICE_HOST").unwrap_or_else(|_| String::default());

        let local_kube_port =
            &std::env::var("KUBERNETES_SERVICE_PORT").unwrap_or_else(|_| String::default());

        match std::env::var("APP_ENV") {
            Ok(app_env) => {
                if app_env == APP_ENV_LOCAL {
                    cacrt_path = local_cacrt_path;
                    namespace = local_namespace;
                    token_path = local_token_path;
                    kube_host = local_kube_host;
                    kube_port = local_kube_port;
                }
            }
            Err(_) => {
                tracing::debug!("Use default kube config, {}", APP_ENV_PRODUCT)
            }
        }

        let cacrt = match std::fs::read(cacrt_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::info!("Failed to read CA certificate: {}", e);
                Vec::<u8>::new()
            }
        };
        let namespace = match std::fs::read_to_string(namespace) {
            Ok(s) => s,
            Err(e) => {
                tracing::info!("Failed to read namespace: {}", e);
                String::new()
            }
        };
        let token = match std::fs::read_to_string(token_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::info!("Failed to read token: {}", e);
                String::new()
            }
        };

        Self {
            kube_host: kube_host.to_string(),
            kube_port: kube_port.to_string(),
            cacrt,
            namespace,
            token,
        }
    }

    pub fn get_tls_connector(&self) -> Result<TlsConnector, anyhow::Error> {
        dotenv::dotenv().ok();
        let cacrt_path = CACRT_PATH;
        let mut builder = native_tls::TlsConnector::builder();
        let local_cacrt_path = &std::env::var("KUBERNETES_CA_CERT_PATH").unwrap_or_else(|_| {
            tracing::debug!("Local nothing, using {}", cacrt_path);
            String::default()
        });
        let cert = std::fs::read_to_string(local_cacrt_path)?;
        let cert = native_tls::Certificate::from_pem(cert.as_bytes())?;
        builder.add_root_certificate(cert);

        Ok(builder.build()?)
    }
}
