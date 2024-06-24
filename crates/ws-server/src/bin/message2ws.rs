use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::{
    HeaderValue, CONNECTION, HOST, SEC_WEBSOCKET_PROTOCOL, SEC_WEBSOCKET_VERSION, UPGRADE,
};

#[tokio::main]
async fn main() {
    let mut request = Request::builder()
        .uri("ws://127.0.0.1:8080//exec?stdout=1&stdin=1&stderr=1&tty=1&command=%2Fbin%2Fsh&command=-i&teamid=1712003096281367424&id=21")
        .body(())
        .unwrap();

    request
        .headers_mut()
        .insert(HOST, HeaderValue::from_static("127.0.0.1:8080"));
    request
        .headers_mut()
        .insert(CONNECTION, HeaderValue::from_static("Upgrade"));
    request
        .headers_mut()
        .insert(UPGRADE, HeaderValue::from_static("websocket"));
    request
        .headers_mut()
        .insert(SEC_WEBSOCKET_VERSION, HeaderValue::from_static("13"));

    // Set random s-w-k
    let mut rng = rand::thread_rng();
    let key: [u8; 16] = rng.gen();
    let key_base64 = general_purpose::STANDARD_NO_PAD.encode(&key);

    request.headers_mut().insert(
        "sec-websocket-key",
        HeaderValue::from_str(&key_base64).unwrap(),
    );

    // Set websocket subprotocol
    request.headers_mut().insert(
        SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static("echo-protocol"),
    );

    let (mut ws_stream, _) = connect_async(request).await.unwrap();
    println!("WebSocket handshake has been successfully completed");

    // 发送心跳消息
    ws_stream
        .send(Message::Text("0".to_string()))
        .await
        .expect("Failed to send heartbeat message");

    // 等待服务器响应
    if let Some(Ok(response)) = ws_stream.next().await {
        if let Message::Text(text) = response {
            println!("{}", text);
            if text == "heartbeat-response" {
                println!("Communication is successful");
            } else {
                println!("Received unexpected message: {}", text);
            }
        } else {
            println!("Received unexpected message: {:?}", response);
        }
    } else {
        println!("Failed to receive response");
    }
}
