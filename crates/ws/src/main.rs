use common::ServiceAccountToken;
use futures_util::{SinkExt as _, StreamExt as _};
use logger::logger_trace::init_logger;
use pod_exec::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

mod pod_exec;

#[tokio::main]
async fn main() {
    init_logger();
    let sat = ServiceAccountToken::new();
    let pod_exec_url = PodExecUrl {
        domain: String::from(&sat.kube_host),
        port: String::from(&sat.kube_port),
        path: PodExecPath {
            base_path: String::from("/api/v1"),
            namespace: String::from("/namespaces/default"),
            pod: String::from("/pods/web-term-559fdfcd89-gndr5"),
            tail_path: String::from("/exec"),
        },
    };
    let pod_exec_params = PodExecParams {
        container: "web-term".to_string(),
        stdin: true,
        stdout: true,
        stderr: true,
        tty: true,
        command: "ls".to_string(),
        pretty: true,
        follow: true,
    };

    let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
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
                tracing::info!("Received text message: {}", text);
            }
            Message::Binary(data) => {
                if let Ok(text) = String::from_utf8(data) {
                    tracing::info!("Received binary message as text: {}", text);
                    if !*is_closed {
                        let text_message = Message::Text(text);
                        if let Err(e) = ws_stream.send(text_message).await {
                            tracing::error!("Failed to send text message: {}", e);
                        }
                    }
                } else {
                    tracing::info!("Failed to convert binary message to text");
                }
            }
            Message::Ping(ping) => {
                if !*is_closed {
                    let pong = Message::Pong(ping);
                    if let Err(e) = ws_stream.send(pong).await {
                        tracing::error!("Failed to send Pong: {}", e);
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

    // tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    // if !*is_closed {
    //     if let Err(e) = ws_stream.close(None).await {
    //         tracing::error!("Failed to close WebSocket connection: {}", e);
    //     } else {
    //         tracing::info!("WebSocket connection closed");
    //     }
    // }
}
