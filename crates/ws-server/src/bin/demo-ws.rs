use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    let request = Request::builder()
        .uri("ws://127.0.0.1:8080//exec?stdout=1&stdin=1&stderr=1&tty=1&command=%2Fbin%2Fsh&command=-i&teamid=1712003096281367424&id=21")
        .header(HOST, "127.0.0.1:8080")
        .header(SEC_WEBSOCKET_KEY, tokio_tungstenite::tungstenite::handshake::client::generate_key())
        .header(SEC_WEBSOCKET_PROTOCOL, "echo-protocol")
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .body(())
        .unwrap();

    let (mut ws_stream, _) = connect_async(request).await.unwrap();
    println!("WebSocket handshake has been successfully completed");

    ws_stream
        .send(Message::Text("0".to_string()))
        .await
        .expect("Failed to send heartbeat message");

    while let Some(Ok(response)) = ws_stream.next().await {
        if let Message::Text(text) = response {
            println!("Receive: {}", text);
        } else {
            println!("Received unexpected message: {:?}", response);
        }
    }

    ws_stream.close(None).await.expect("Failed to close WebSocket connection");
    println!("WebSocket connection closed");
}
