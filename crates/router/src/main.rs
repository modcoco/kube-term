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
use kube::ServiceAccountToken;
use logger::logger_trace::init_logger;
use pod_exec::{
    connector::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl},
    msg_handle::handle_websocket,
};

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

async fn _handle_socket(mut socket: WebSocket) {
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

async fn handle_socket(mut socket: WebSocket) {
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

    // 创建异步消息通道
    let (tx_web, mut rx_web) = mpsc::channel::<Message>(100);
    let (tx_ws, mut rx_ws) = mpsc::channel(100);

    let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
    match conn {
        Ok(mut ws_stream) => {
            let mut closed = false;
            tokio::spawn(async move {
                handle_websocket(&mut ws_stream, &mut rx_web, &tx_ws, &mut closed).await;
            });
        }
        Err(err) => {
            tracing::error!("ERROR, {}", err)
        }
    };

    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            // tracing::info!("{:?}", &msg);
            msg
        } else {
            // client disconnected
            tracing::info!("Client disconnected, the msg isn't ok");
            return;
        };
        // 将收到的消息发送到管道
        if tx_web.send(msg).await.is_err() {
            tracing::info!("Failed to send message to channel");
            return;
        }

        // 从管道接收消息
        if let Some(resp_msg) = rx_ws.recv().await {
            tracing::info!("{}", resp_msg);
            // 将从管道中获取的消息重新发送给客户端
            let resp_msg = Message::Text(resp_msg);
            if socket.send(resp_msg).await.is_err() {
                // client disconnected
                tracing::info!("Client disconnected, failed to send message");
                return;
            }
        }
    }
}
