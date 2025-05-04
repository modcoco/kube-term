use common::tokio::net::TcpStream;
use common::tokio::sync::mpsc;
use common::tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use common::{axum, base64, futures_util, serde_json, tokio, tracing};
use common::{
    tokio::io::{stdin, AsyncBufReadExt as _, BufReader},
    tokio_tungstenite,
};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

const STD_INPUT_PREFIX: u8 = 0x00;
const STD_OUTPUT_PREFIX_NORMAL: u8 = 0x01;
const STD_OUTPUT_PREFIX_ERR: u8 = 0x02;
// const CR: u8 = 0x0D;
const LF: u8 = 0x0A;

#[derive(Debug, Serialize, Deserialize, Default)]
struct Data {
    rows: u16,
    columns: u16,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ResizeMessage {
    r#type: String,
    data: Data,
}

// e.g. {"Width":80,"Height":24}
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct TerminalSize {
    width: u16,
    height: u16,
}

pub async fn stdin_reader(tx: mpsc::Sender<String>) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdin());
        let mut line = String::new();

        while reader.read_line(&mut line).await.is_ok() {
            tracing::debug!("Read line from stdin: {}", line.trim());
            if tx.send(line.clone()).await.is_err() {
                break;
            }
            line.clear();
        }
    });
}

pub trait MessageHandler {
    fn handle_message(self) -> String;
}

impl MessageHandler for axum::extract::ws::Message {
    fn handle_message(self) -> String {
        match self {
            axum::extract::ws::Message::Text(text) => text,
            _ => "".to_string(),
        }
    }
}

impl MessageHandler for String {
    fn handle_message(self) -> String {
        self
    }
}

pub async fn handle_websocket<M>(
    kube_ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    rx_web: &mut mpsc::Receiver<M>,
    tx_kube: &mpsc::Sender<String>,
    is_closed: &mut bool,
    debug: Option<bool>,
) where
    M: MessageHandler + 'static,
{
    let mut chat_no: i32 = Default::default();
    let mut step = Default::default();
    loop {
        tokio::select! {
            Some(input) = rx_web.recv() => {
                let input: String = input.handle_message();
                step = 0;
                chat_no += 1;

                let resize_msg = build_resize_msg(input.clone());
                let ascii_msg = build_ascii_msg(input, resize_msg, debug);

                let message = Message::Binary(ascii_msg);
                if let Err(err) = kube_ws_stream.send(message).await {
                    tracing::error!("Failed to send binary message to kube ws: {}", err);
                    *is_closed = true;
                }
            },
            Some(Ok(msg)) = kube_ws_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        tracing::info!("Received text message: {}", text);
                    }
                    Message::Binary(data) => {
                        tracing::debug!("chat_no {}", &chat_no);
                        if handle_binary_to_kube_channel(data, tx_kube, step, debug).await {
                            step += 1;
                        }
                    }
                    Message::Ping(ping) => {
                        tracing::info!("Received Ping message");
                        let pong = Message::Pong(ping);
                        if let Err(err) = kube_ws_stream.send(pong).await {
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

pub async fn handle_binary_to_kube_channel(
    data: Vec<u8>,
    tx_kube: &mpsc::Sender<String>,
    step: i32,
    cmd_debug: Option<bool>,
) -> bool {
    let data_exist = !data.is_empty();
    let data_prefix = data[0];
    let data_value = data[1..].to_vec();

    if data_exist {
        match data_prefix {
            STD_OUTPUT_PREFIX_NORMAL => {
                let msg_ascii = data_value;
                tracing::debug!("step {}, received org: {:?}", step, msg_ascii);

                let msg_ascii = cmd_debug.map_or(msg_ascii.clone(), |debug| {
                    if debug {
                        local_dev_cmd_auxiliary_display(step, msg_ascii)
                    } else {
                        msg_ascii
                    }
                });

                if !msg_ascii.is_empty() {
                    let kube_msg = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, data);
                    let kube_msg = format!("1{kube_msg}");
                    if tx_kube.send(kube_msg).await.is_err() {
                        tracing::error!("Failed to send message to kube chanel");
                    }
                }
            }
            STD_OUTPUT_PREFIX_ERR => {
                if let Ok(msg) = String::from_utf8(data_value) {
                    tracing::info!("Received stderr: {}", msg);
                } else {
                    tracing::info!("Failed to convert stderr to text");
                }
            }
            _ => {
                tracing::info!("Unknown binary message prefix: {:?}", data_prefix);
            }
        }
        return true;
    }
    false
}

fn local_dev_cmd_auxiliary_display(step: i32, mut msg_ascii: Vec<u8>) -> Vec<u8> {
    // \x1b[?2004l\r
    // 13, 10, 27, 91, 63, 50, 48, 48, 52, 108, 13
    if step == 0 {
        if let Some(pos) = msg_ascii
            .windows(11)
            .position(|window| window == [13, 10, 27, 91, 63, 50, 48, 48, 52, 108, 13])
        {
            msg_ascii = msg_ascii.split_off(pos + 11);
        };
        // tracing::info!("step {}, received fix: {:?}", step, msg_ascii);
    }
    // \x1B[?2004h
    // 27, 91, 63, 50, 48, 48, 52, 104
    if msg_ascii.starts_with(&[27, 91, 63, 50, 48, 48, 52, 104]) {
        tracing::debug!("remove colir");
        msg_ascii = msg_ascii[8..].to_vec();
    }
    msg_ascii
}

fn build_resize_msg(mut client_resize_msg: String) -> String {
    if !client_resize_msg.starts_with('9') {
        return Default::default();
    }
    if !client_resize_msg.is_empty() {
        client_resize_msg.remove(0);
    }
    let client_msg = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, client_resize_msg)
        .unwrap_or_default();
    let client_msg: String = client_msg.iter().map(|&c| c as char).collect();
    let client_msg: ResizeMessage = serde_json::from_str(&client_msg).unwrap_or_default();
    tracing::debug!("client resize msg => {:?}", client_msg);

    let terminal_size = TerminalSize {
        width: client_msg.data.columns,
        height: client_msg.data.rows,
    };
    let resize_msg = serde_json::to_string(&terminal_size).unwrap_or_default();
    tracing::debug!("{}", resize_msg);
    resize_msg
}

fn build_ascii_msg(mut input: String, resize_msg: String, debug: Option<bool>) -> Vec<u8> {
    let mut buffer = vec![];

    if !resize_msg.is_empty() {
        buffer.push(0x04);
        let input = resize_msg.as_bytes();
        buffer.extend_from_slice(input);
        return buffer;
    }

    if debug.is_none_or(|debug| !debug) {
        buffer.push(STD_INPUT_PREFIX);
        input = input.chars().skip(1).collect();
        let input =
            base64::Engine::decode(&base64::prelude::BASE64_STANDARD, input).unwrap_or_default();
        buffer.extend_from_slice(&input);
    } else {
        let input = input.trim().chars().collect::<String>();
        let input = input.as_bytes();
        buffer.extend_from_slice(input);
        // buffer.push(CR);
        buffer.push(LF);
    }

    tracing::debug!("=> sending message to kube: {:?}", buffer);
    buffer
}
