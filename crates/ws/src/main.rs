use common::futures_util;
use common::tokio::io::AsyncBufReadExt as _;
use common::tokio::net::TcpStream;
use common::{tokio, tokio_tungstenite, tracing};
use futures_util::{SinkExt as _, StreamExt as _};
use kube::ServiceAccountToken;
use logger::logger_trace::init_logger;
use pod_exec::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl};
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
        command: "bash".to_string(),
        pretty: true,
        follow: true,
    };

    let conn: Result<WebSocketStream<MaybeTlsStream<TcpStream>>, common::anyhow::Error> =
        pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
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
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    // Spawn a task to read from stdin and send to the channel
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();

        while reader.read_line(&mut line).await.is_ok() {
            // tracing::info!("Read line from stdin: {}", line.trim());
            if tx.send(line.clone()).await.is_err() {
                break;
            }
            line.clear();
        }
    });

    loop {
        tokio::select! {
            Some(input) = rx.recv() => {
                let input = input.trim().chars().collect::<String>();

                let mut buffer = vec![0x00];
                buffer.extend_from_slice(input.as_bytes());
                buffer.push(0x0A);

                tracing::info!("------Sending message to WebSocket: {:?}", buffer);
                let message = Message::Binary(buffer);
                if let Err(err) = ws_stream.send(message).await {
                    tracing::error!("Failed to send binary message to WebSocket: {}", err);
                    *is_closed = true;
                }
            },
            Some(Ok(msg)) = ws_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        tracing::info!("Received text message: {}", text);
                    }
                    Message::Binary(data) => {
                        // tracing::info!("Received binary message: {:?}", data);
                        if !data.is_empty() {
                            match data[0] {
                                0x01 => {
                                    // 处理标准输出消息
                                    if let Ok(text) = String::from_utf8(data[1..].to_vec()) {
                                        tracing::info!("Received stdout: {}", text);
                                    } else {
                                        tracing::info!("Failed to convert stdout to text");
                                    }
                                }
                                0x02 => {
                                    // 处理标准错误输出消息
                                    if let Ok(text) = String::from_utf8(data[1..].to_vec()) {
                                        tracing::info!("Received stderr: {}", text);
                                    } else {
                                        tracing::info!("Failed to convert stderr to text");
                                    }
                                }
                                _ => {
                                    tracing::info!("Unknown binary message prefix: {:?}", data[0]);
                                }
                            }
                        } else {
                            tracing::info!("Received empty binary message");
                        }
                    }
                    Message::Ping(ping) => {
                        tracing::info!("Received Ping message");
                        let pong = Message::Pong(ping);
                        if let Err(err) = ws_stream.send(pong).await {
                            tracing::error!("Failed to send Pong: {}", err);
                        }
                    }
                    Message::Close(_) => {
                        tracing::info!("Received Close message");
                        *is_closed = true;
                    }
                    _ => {}
                }
            },
        }
    }
}
