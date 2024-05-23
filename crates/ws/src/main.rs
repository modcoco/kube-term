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

async fn connect() -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    tracing::debug!("attempting connection");
    let request = Request::builder()
        .uri("wss://127.0.0.1:6443/api/v1/namespaces/cn/pods/custom-image-pod-81367424-00000021-job-0/exec?container=custom&stdin=true&stdout=true&stderr=true&tty=true&command=ls&pretty=true&follow=true")
        .header(HOST, "127.0.0.1:6443")
        .header("Origin", "https://127.0.0.1:6443")
        .header(SEC_WEBSOCKET_KEY, tokio_tungstenite::tungstenite::handshake::client::generate_key())
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .header("Authorization", "Bearer eyJhbGciOiJSUzI1NiIsImtpZCI6IkloUk1rOVZ6cGg1NjFSdEJqOWw4V3NTWFVMeUxPdXoxTGhZM2JYRkpSV2MifQ.eyJpc3MiOiJrdWJlcm5ldGVzL3NlcnZpY2VhY2NvdW50Iiwia3ViZXJuZXRlcy5pby9zZXJ2aWNlYWNjb3VudC9uYW1lc3BhY2UiOiJjbiIsImt1YmVybmV0ZXMuaW8vc2VydmljZWFjY291bnQvc2VjcmV0Lm5hbWUiOiJyZWFkLXNhLXRva2VuLXRva2VuLWduYmJkIiwia3ViZXJuZXRlcy5pby9zZXJ2aWNlYWNjb3VudC9zZXJ2aWNlLWFjY291bnQubmFtZSI6InJlYWQtc2EtdG9rZW4iLCJrdWJlcm5ldGVzLmlvL3NlcnZpY2VhY2NvdW50L3NlcnZpY2UtYWNjb3VudC51aWQiOiIxZjQ0OGRlMi0zMWZhLTQzZTYtOGMxYS02ZTdhMDM1Y2QzYTUiLCJzdWIiOiJzeXN0ZW06c2VydmljZWFjY291bnQ6Y246cmVhZC1zYS10b2tlbiJ9.A2uW9R2XZ5L0aECj4eVpZ-fmQngtY_w7USeROmFllx4nOA-gmSKHq1b3bm51r9O_igiIgaL69tH1W4bktIhl1cmrJRKIp_5C4lWZLWJO9A02dVgy0VHXHkOL41JFlTeq0fc_vlpPLfXKl_JEnL0CUdOzC5WNLf3ZtkKgQ8_D2-7dd7qoYArPvclqSdPjxU90wxpIEyctD_AgfVlMnpZZR2VnfjvIBuVE6e8LwmR89SyRnKqp24PYYFlQZInFioc4Uqg6MF_o0u51BPCUR9OUOQweqyW1PnTbVZOZFBmZa64qXfyRAMGp3IIhnGg7ARvihH8uU3Pc5bdXKvLfNLqSYQ")
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
