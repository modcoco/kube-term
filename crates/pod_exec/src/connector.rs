use common::anyhow::{self, Result};
use common::axum::extract::RawPathParams;
use common::{tokio, tokio_tungstenite, tracing};
use kube::ServiceAccountToken;
use std::fmt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use util::url_https_builder;

#[derive(Debug, Default)]
pub struct ContainerCoords {
    pub namespace: String,
    pub pod: String,
    pub container: String,
}

impl ContainerCoords {
    pub fn populate_from_raw_path_params(mut self, raw_path_params: &RawPathParams) -> Self {
        for (key, value) in raw_path_params.iter() {
            match key {
                "namespace" => value.clone_into(&mut self.namespace),
                "pod" => value.clone_into(&mut self.pod),
                "container" => value.clone_into(&mut self.container),
                _ => todo!(),
            }
        }
        self
    }
}

#[derive(Debug, Default)]
pub struct PodExecUrl {
    pub domain: String,
    pub port: String,
    pub path: PodExecPath,
}
impl PodExecUrl {
    pub fn get_exec_url(&self, kube_host: &str, kube_port: &str, coords: &ContainerCoords) -> Self {
        Self {
            domain: String::from(kube_host),
            port: String::from(kube_port),
            path: PodExecPath {
                base_path: String::from("/api/v1"),
                namespace: format!("/namespaces/{}", coords.namespace),
                pod: format!("/pods/{}", coords.pod),
                tail_path: String::from("/exec"),
            },
        }
    }
}

impl PodExecUrl {
    pub fn format(&self) -> String {
        format!("wss://{}:{}{}", self.domain, self.port, self.path)
    }
}

#[derive(Debug, Default)]
pub struct PodExecPath {
    pub base_path: String,
    pub namespace: String,
    pub pod: String,
    pub tail_path: String,
}

impl fmt::Display for PodExecPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.base_path, self.namespace, self.pod, self.tail_path
        )
    }
}

#[derive(Debug, Default)]
pub struct PodExecParams {
    pub container: String,
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,
    pub tty: bool,
    pub command: String,
    pub pretty: bool,
    pub follow: bool,
}

impl PodExecParams {
    pub fn format(&self) -> String {
        format!(
            "?container={}&stdin={}&stdout={}&stderr={}&tty={}&command={}&pretty={}&follow={}",
            self.container,
            self.stdin,
            self.stdout,
            self.stderr,
            self.tty,
            self.command,
            self.pretty,
            self.follow
        )
    }
    pub fn get_pod_exec_params(&self, coords: &ContainerCoords) -> Self {
        Self {
            container: coords.container.clone(),
            stdin: true,
            stdout: true,
            stderr: true,
            tty: true,
            command: "env&env=TERM%3Dxterm&command=COLUMNS%3D800&command=LINES%3D10&command=bash"
                .to_string(),
            pretty: true,
            follow: true,
        }
    }
}

pub async fn pod_exec_connector(
    sat: &ServiceAccountToken,
    pod_exec_url: &PodExecUrl,
    pod_exec_params: &PodExecParams,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, anyhow::Error> {
    tracing::debug!("attempting connection");
    let kubernetes_token = &sat.token;
    let kubernetes_cacrt = sat.get_tls_connector()?;
    let websocket_url = format!("{}{}", pod_exec_url.format(), pod_exec_params.format());
    let kube_domain = url_https_builder(&sat.kube_host, &sat.kube_port, None);

    let request = Request::builder()
        .uri(websocket_url)
        .header(HOST, format!("{}:{}", &sat.kube_host, &sat.kube_port))
        .header("Origin", kube_domain)
        .header(
            SEC_WEBSOCKET_KEY,
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .header("Authorization", format!("Bearer {}", kubernetes_token))
        .header(SEC_WEBSOCKET_PROTOCOL, "channel.k8s.io")
        .body(())?;

    let connector = Connector::NativeTls(kubernetes_cacrt);
    match connect_async_tls_with_config(request, None, true, Some(connector)).await {
        Ok((conn, _)) => {
            tracing::info!("Successfully connected!");
            Ok(conn)
        }
        Err(err) => {
            tracing::info!("Failed to connect: {}", err);
            Err(Box::new(err).into())
        }
    }
}
