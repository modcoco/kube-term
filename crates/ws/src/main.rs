use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ServerName};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "ws-server=debug"); // project_name=debug,tower_http=debug
    }
    tracing_subscriber::fmt::init();
    tracing::info!("init");

    let conn: std::prelude::v1::Result<WebSocketStream<MaybeTlsStream<TcpStream>>, anyhow::Error> =
        connect().await;
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
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
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

struct IgnoreAllCertificateSecurity;

impl ServerCertVerifier for IgnoreAllCertificateSecurity {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

#[derive(Debug)]
struct UrlComponents {
    domain: String,
    port: String,
    path: String,
}

impl UrlComponents {
    fn format(&self) -> String {
        format!("wss://{}:{}{}", self.domain, self.port, self.path)
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

async fn connect() -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    tracing::debug!("attempting connection");

    let url_components = UrlComponents {
        domain: "192.168.2.4".to_owned(),
        port: "6443".to_owned(),
        path: "/api/v1/namespaces/ns/pods/podename/".to_string(),
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

    let websocket_url = format!("{}{}", url_components.format(), query_params.format());

    let request = Request::builder()
        .uri(websocket_url)
        .header(HOST, "192.168.2.4:6443")
        .header("Origin", "https://192.168.2.4:6443")
        .header(
            SEC_WEBSOCKET_KEY,
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .header("Authorization", "Bearer eyJhbGciOiJ")
        .body(())
        .unwrap();
    let connector = Connector::Rustls(Arc::new(
        rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(IgnoreAllCertificateSecurity))
            .with_no_client_auth(),
    ));
    let (conn, _) = connect_async_tls_with_config(request, None, true, Some(connector)).await?;
    tracing::debug!("connected");
    Ok(conn)
}
