use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use common::{axum, tokio, tracing};
use logger::logger_trace::init_logger;

#[tokio::main]
async fn main() {
    init_logger();
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/ws", get(handler));

    tracing::info!("start web server...");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
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
