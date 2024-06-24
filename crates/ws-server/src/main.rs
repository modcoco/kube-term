use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::debug;
use native_tls::TlsConnector;
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

async fn connect() -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    debug!("attempting connection");
    let request = Request::builder()
        .uri("wss://ubuntu:6443/api/v1/namespaces/default/pods/web-term-559fdfcd89-gndr5/exec?container=web-term&stdin=true&stdout=true&stderr=true&tty=true&command=ls&pretty=true&follow=true")
        .header(HOST, "ubuntu:6443")
        .header("Origin", "https://ubuntu:6443")
        .header(SEC_WEBSOCKET_KEY, tokio_tungstenite::tungstenite::handshake::client::generate_key())
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .header(SEC_WEBSOCKET_PROTOCOL, "channel.k8s.io")
        .header("Authorization", "Bearer ")
        .body(())
        .unwrap();

    let connector = Connector::NativeTls(get_tls_connector()?);
    let (conn, _) = connect_async_tls_with_config(request, None, true, Some(connector)).await?;
    debug!("connected");
    Ok(conn)
}

pub fn get_tls_connector() -> Result<TlsConnector, anyhow::Error> {
    let mut builder = native_tls::TlsConnector::builder();
    let cert = std::fs::read_to_string("/home/mahongqin/.k8s/ca.crt")?;
    let cert = native_tls::Certificate::from_pem(cert.as_bytes())?;
    builder.add_root_certificate(cert);

    Ok(builder.build().unwrap())
}
