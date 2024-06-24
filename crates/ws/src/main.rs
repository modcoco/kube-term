use common::anyhow::Result;
use common::PodSecrets;
use futures_util::{SinkExt, StreamExt};
use std::fmt;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async_tls_with_config, MaybeTlsStream, WebSocketStream};

#[tokio::main]
pub async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "ws-server=debug"); // project_name=debug,tower_http=debug
    }
    tracing_subscriber::fmt::init();
    tracing::info!("init");

    let conn: std::result::Result<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TlsStream<TcpStream>>>,
        anyhow::Error,
    > = connect().await;
    match conn {
        Ok(mut ws_stream) => {
            let mut closed = false;
            handle_websocket(&mut ws_stream, &mut closed).await;
        }
        Err(_) => {
            println!("ERROR")
        }
    };
}

async fn handle_websocket(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TlsStream<TcpStream>>>,
    is_closed: &mut bool,
) {
    while let Some(Ok(msg)) = ws_stream.next().await {
        if *is_closed {
            break;
        }
        match msg {
            Message::Text(text) => {
                println!("Received text message: {}", text);
            }
            Message::Binary(data) => {
                if let Ok(text) = String::from_utf8(data) {
                    println!("Received binary message as text: {}", text);
                    if !*is_closed {
                        let text_message = Message::Text(text);
                        if let Err(e) = ws_stream.send(text_message).await {
                            eprintln!("Failed to send text message: {}", e);
                        }
                    }
                } else {
                    println!("Failed to convert binary message to text");
                }
            }
            Message::Ping(ping) => {
                if !*is_closed {
                    let pong = Message::Pong(ping);
                    if let Err(e) = ws_stream.send(pong).await {
                        eprintln!("Failed to send Pong: {}", e);
                    }
                }
            }
            Message::Pong(_) => todo!(),
            Message::Close(_) => {
                // 处理关闭消息
                break;
            }
            Message::Frame(_) => todo!(),
        }
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    if !*is_closed {
        if let Err(e) = ws_stream.close(None).await {
            eprintln!("Failed to close WebSocket connection: {}", e);
        } else {
            println!("WebSocket connection closed");
        }
    }
}

#[derive(Debug)]
struct UrlComponents {
    domain: String,
    port: String,
    path: PathComponents,
}

impl UrlComponents {
    fn format(&self) -> String {
        format!("wss://{}:{}{}", self.domain, self.port, self.path)
    }
}

#[derive(Debug)]
struct PathComponents {
    version: String,
    namespace: String,
    resource_type: String,
    resource_name: String,
}

impl fmt::Display for PathComponents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.version, self.namespace, self.resource_type, self.resource_name
        )
    }
}

#[derive(Debug)]
struct QueryParamsK8sTerm {
    container: String,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
    command: String,
    pretty: bool,
    follow: bool,
}

impl QueryParamsK8sTerm {
    fn format(&self) -> String {
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
}

async fn connect() -> Result<WebSocketStream<MaybeTlsStream<TlsStream<TcpStream>>>, anyhow::Error> {
    tracing::debug!("attempting connection");
    let ps = PodSecrets::new();
    let kubernetes_token = ps.token;
    // let kubernetes_cert: Certificate = Certificate::from_pem(&ps.cacrt)?;

    let components = UrlComponents {
        domain: String::from("ubuntu"),
        port: String::from("6443"),
        path: PathComponents {
            version: String::from("/api/v1"),
            namespace: String::from("/namespaces/default"),
            resource_type: String::from("/pods"),
            resource_name: String::from("/web-term-85b6756ff7-b42hk"),
        },
    };

    let query_params = QueryParamsK8sTerm {
        container: "custom".to_string(),
        stdin: true,
        stdout: true,
        stderr: true,
        tty: true,
        command: "ls".to_string(),
        pretty: true,
        follow: true,
    };

    let websocket_url = format!("{}{}", components.format(), query_params.format());

    let request = Request::builder()
        .uri(websocket_url)
        .header(HOST, "ubuntu:6443")
        .header("Origin", "https://ubuntu:6443")
        .header(
            SEC_WEBSOCKET_KEY,
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .header("Authorization", format!("Bearer {}", kubernetes_token))
        .header(SEC_WEBSOCKET_PROTOCOL, "channel.k8s.io")
        .body(())
        .unwrap();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    // 创建 Rustls 客户端配置并加载根证书
    let mut root_cert_store = rustls::RootCertStore::empty();
    let cafile = "/home/mahongqin/.k8s/ca.crt";
    let mut buf = Vec::new();
    let mut file = File::open(cafile).await?;
    file.read_to_end(&mut buf).await?;
    let mut pem = std::io::Cursor::new(buf);
    let certs = rustls_pemfile::certs(&mut pem).collect::<Result<Vec<_>, _>>()?;
    for cert in certs {
        _ = root_cert_store.add(cert);
    }

    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
    let tcp_stream = TcpStream::connect("ubuntu:6443").await?;
    let domain = rustls::pki_types::ServerName::try_from("ubuntu").unwrap();
    let tls_stream: TlsStream<TcpStream> = connector.connect(domain, tcp_stream).await?;

    let (ws_stream, _) = client_async_tls_with_config(request, tls_stream, None, None)
        .await
        .expect("Failed to connect");
    tracing::info!("connected");

    Ok(ws_stream)
}
