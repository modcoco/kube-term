use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use common::{
    axum::{self, extract::ws::Message},
    tokio::{self, net::TcpListener, sync::mpsc},
    tracing,
};
use logger::logger_trace::init_logger;

#[tokio::main]
async fn main() {
    init_logger();
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/ws", get(handler));

    tracing::info!("start web server...");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            tracing::info!("{:?}", &msg);
            msg
        } else {
            // client disconnected
            tracing::info!("client disconnected, the msg is't ok");
            return;
        };

        if socket.send(msg).await.is_err() {
            // client disconnected
            tracing::info!("client disconnected, user has disconnect");
            return;
        }
    }
}

async fn _handle_socket_with_chennl(mut socket: WebSocket) {
    // 创建异步消息通道
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            tracing::info!("{:?}", &msg);
            msg
        } else {
            // client disconnected
            tracing::info!("Client disconnected, the msg isn't ok");
            return;
        };

        // 将收到的消息发送到管道
        if tx.send(msg.clone()).await.is_err() {
            tracing::info!("Failed to send message to channel");
            return;
        }

        // 从管道接收消息
        if let Some(resp_msg) = rx.recv().await {
            // 将从管道中获取的消息重新发送给客户端
            if socket.send(resp_msg).await.is_err() {
                // client disconnected
                tracing::info!("Client disconnected, failed to send message");
                return;
            }
        }
    }
}
