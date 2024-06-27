use common::tokio::net::TcpStream;
use common::tokio::sync::mpsc;
use common::tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use common::{futures_util, tokio, tracing};
use common::{
    tokio::io::{stdin, AsyncBufReadExt as _, BufReader},
    tokio_tungstenite,
};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;

pub async fn stdin_reader(tx: mpsc::Sender<String>) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdin());
        let mut line = String::new();

        while reader.read_line(&mut line).await.is_ok() {
            // tracing::info!("Read line from stdin: {}", line.trim());
            if tx.send(line.clone()).await.is_err() {
                break;
            }
            line.clear();
        }
    });
}

pub async fn handle_websocket(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    rx: &mut mpsc::Receiver<String>,
    tx_ws: &mpsc::Sender<String>,
    is_closed: &mut bool,
) {
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
                        handle_binary(data, tx_ws).await;
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

pub async fn handle_binary(data: Vec<u8>, tx_ws: &mpsc::Sender<String>) {
    if !data.is_empty() {
        match data[0] {
            0x01 => {
                // 处理标准输出消息
                if let Ok(text) = String::from_utf8(data[1..].to_vec()) {
                    // tracing::info!("Received stdout: {}", text);
                    if tx_ws.send(text).await.is_err() {
                        tracing::error!("Failed to send message to main");
                    }
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
