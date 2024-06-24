use common::anyhow::Result;
use common::constants::COLON;
use common::{url_https_builder, PodSecrets};
use futures_util::{SinkExt as _, StreamExt as _};
use logger::logger_trace::init_logger;
use std::fmt;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

#[tokio::main]
async fn main() {
    init_logger();
    tracing::info!("init");

    let conn: std::prelude::v1::Result<WebSocketStream<MaybeTlsStream<TcpStream>>, anyhow::Error> =
        connect().await;
    match conn {
        Ok(mut ws_stream) => {
            let mut closed = false;
            handle_websocket(&mut ws_stream, &mut closed).await;
        }
        Err(err) => {
            println!("ERROR, {}", err)
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
    pod: String,
}

impl fmt::Display for PathComponents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.version, self.namespace, self.pod)
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

async fn connect() -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, anyhow::Error> {
    tracing::debug!("attempting connection");
    let ps = PodSecrets::new();
    let kubernetes_token = &ps.token;

    let components = UrlComponents {
        domain: String::from(&ps.kube_host),
        port: String::from(&ps.kube_port),
        path: PathComponents {
            version: String::from("/api/v1"),
            namespace: String::from("/namespaces/default"),
            pod: String::from("/pods/web-term-559fdfcd89-gndr5"),
        },
    };

    let query_params = QueryParamsK8sTerm {
        container: "web-term".to_string(),
        stdin: true,
        stdout: true,
        stderr: true,
        tty: true,
        command: "ls".to_string(),
        pretty: true,
        follow: true,
    };

    let websocket_url = format!("{}/exec{}", components.format(), query_params.format());
    let kube_domain = url_https_builder(&ps.kube_host, &ps.kube_port, "");
    let request = Request::builder()
        .uri(websocket_url)
        .header(HOST, format!("{}{}{}", &ps.kube_host, COLON, &ps.kube_port))
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

    let connector = Connector::NativeTls(ps.get_tls_connector()?);
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
