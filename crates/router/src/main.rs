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

        // Process the message and decide to send multiple responses
        if let Message::Text(text) = msg {
            let responses = vec![
                Message::Text(format!("Received: {}", text)),
                Message::Text("This is another message".into()),
                Message::Text("And yet another message".into()),
            ];

            for response in responses {
                if socket.send(response).await.is_err() {
                    // client disconnected
                    tracing::info!("client disconnected, user has disconnected");
                    return;
                }
            }
        } else {
            // Handle other message types if needed
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
    let (tx_kube, mut rx_kube) = mpsc::channel(100);

    let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
    match conn {
        Ok(mut ws_stream) => {
            let mut closed = false;
            tokio::spawn(async move {
                handle_websocket(&mut ws_stream, &mut rx_web, &tx_kube, &mut closed).await;
            });
        }
        Err(err) => {
            tracing::error!("ERROR, {}", err)
        }
    };

    loop {
        tokio::select! {
            Some(client_msg) = socket.recv() => {
                let client_msg = if let Ok(client_msg) = client_msg {
                    tracing::info!("Received from client: {:?}", client_msg);
                    client_msg
                } else {
                    // client disconnected
                    tracing::info!("Client disconnected, the msg isn't ok");
                    return;
                };

                // 将收到的kube消息发送到web
                if tx_web.send(client_msg).await.is_err() {
                    tracing::info!("Failed to send message to channel");
                }
            },
            Some(kube_msg) = rx_kube.recv() => {
                handler_kube_recv(&kube_msg, &mut socket).await;
            }
        }
    }
}

async fn handler_kube_recv(kube_msg: &str, socket: &mut WebSocket) {
    tracing::info!("Received from kubernetes: {}", kube_msg);
    let kube_msg = Message::Text(kube_msg.to_owned());
    if socket.send(kube_msg).await.is_err() {
        // client disconnected
        tracing::info!("Client disconnected, failed to send message");
    }
}
