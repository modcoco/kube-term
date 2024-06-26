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
        command: "sh".to_string(),
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

async fn handle_websocket_test(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    is_closed: &mut bool,
) {
    // 使用MPSC创建一个共享的消息队列
    let (mut tx, mut rx) = tokio::sync::mpsc::channel(100);

    // 单独的任务来读取用户输入
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();

        loop {
            match reader.read_line(&mut line).await {
                Ok(_) => {
                    if (tx.send(line.clone()).await).is_err() {
                        break;
                    }
                    line.clear();
                }
                Err(err) => {
                    eprintln!("Failed to read from stdin: {}", err);
                    break;
                }
            }
        }
    });

    while let Some(Ok(msg)) = ws_stream.next().await {
        if *is_closed {
            break;
        }
        match msg {
            Message::Text(text) => {
                tracing::info!("Received text message: {}", text);
            }
            Message::Binary(data) => {
                tracing::info!("----{:?}", data);
                if let Ok(text) = String::from_utf8(data) {
                    tracing::info!("Received binary message as text: {}", text);
                    if !*is_closed {
                        let text_message = Message::Text(text);
                        if let Err(err) = ws_stream.send(text_message).await {
                            tracing::error!("Failed to send text message: {}", err);
                        }
                    }
                } else {
                    tracing::info!("Failed to convert binary message to text");
                }
            }
            Message::Ping(ping) => {
                if !*is_closed {
                    let pong = Message::Pong(ping);
                    if let Err(err) = ws_stream.send(pong).await {
                        tracing::error!("Failed to send Pong: {}", err);
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

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    if !*is_closed {
        if let Err(e) = ws_stream.close(None).await {
            tracing::error!("Failed to close WebSocket connection: {}", e);
        } else {
            tracing::info!("WebSocket connection closed");
        }
    }
}

async fn handle_websocket_test2(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    is_closed: &mut bool,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();

        while (reader.read_line(&mut line).await).is_ok() {
            tracing::info!("test");
            if (tx.send(line.clone()).await).is_err() {
                break;
            }
            line.clear();
        }
    });

    while let Some(input) = rx.recv().await {
        tracing::info!("Sending message to WebSocket: {}", input);
        let message = Message::Text(input);
        if let Err(err) = ws_stream.send(message).await {
            tracing::error!("Failed to send message to WebSocket: {}", err);
        }

        if *is_closed {
            break;
        }
    }

    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => {
                tracing::info!("Received text message: {}", text);
            }
            Message::Binary(data) => {
                tracing::info!("----{:?}", data);
                if let Ok(text) = String::from_utf8(data) {
                    tracing::info!("Received binary message as text: {}", text);
                    if !*is_closed {
                        let text_message = Message::Text(text);
                        if let Err(err) = ws_stream.send(text_message).await {
                            tracing::error!("Failed to send text message: {}", err);
                        }
                    }
                } else {
                    tracing::info!("Failed to convert binary message to text");
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
                *is_closed = true;
                break;
            }
            _ => {}
        }
    }

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    if !*is_closed {
        if let Err(e) = ws_stream.close(None).await {
            tracing::error!("Failed to close WebSocket connection: {}", e);
        } else {
            tracing::info!("WebSocket connection closed");
        }
    }
}

async fn handle_websocket_ok(
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
                tracing::info!("Sending message to WebSocket: {}", input.trim());

                // 将输入内容转换为字节向量
                let mut buffer = vec![0x00]; // 添加前缀 0x00
                buffer.extend_from_slice(input.trim().as_bytes());
                let message = Message::Binary(buffer);

                if let Err(err) = ws_stream.send(message).await {
                    tracing::error!("Failed to send binary message to WebSocket: {}", err);
                    *is_closed = true;
                }
            }
            Some(Ok(msg)) = ws_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        tracing::info!("Received text message: {}", text);
                    }
                    Message::Binary(data) => {
                        tracing::info!("Received binary message: {:?}", data);
                        // 检查接收到的二进制消息是否包含前缀 0x01（表示服务器的响应）
                        if !data.is_empty() && data[0] == 0x01 {
                            // 处理服务器的响应消息
                            if let Ok(text) = String::from_utf8(data[1..].to_vec()) {
                                tracing::info!("Received server response as text: {}", text);
                            } else {
                                tracing::info!("Failed to convert binary message to text");
                            }
                        } else {
                            tracing::info!("Unexpected binary message: {:?}", data);
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
                tracing::info!("Sending message to WebSocket: {}", input.trim());

                // Skip first character
                let input = input.trim().chars().collect::<String>();

                let input_base64 = base64::encode(&input);

                tracing::info!("Sending input_base64 encoded message to WebSocket: {:?}", input_base64);

                let mut buffer = vec![0x00];
                buffer.extend_from_slice(input_base64.as_bytes());

                tracing::info!("Sending base64 encoded message to WebSocket: {:?}", buffer);

                let message = Message::Binary(buffer);

                if let Err(err) = ws_stream.send(message).await {
                    tracing::error!("Failed to send binary message to WebSocket: {}", err);
                    *is_closed = true;
                }
            },
            // 从 WebSocket 接收消息并处理
            Some(Ok(msg)) = ws_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        tracing::info!("Received text message: {}", text);
                    }
                    Message::Binary(data) => {
                        tracing::info!("Received binary message: {:?}", data);
                        // 检查接收到的二进制消息是否包含前缀 0x01 或 0x02
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
