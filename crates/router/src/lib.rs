use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use common::{
    axum::{self, extract::ws::Message},
    base64,
    tokio::{self, net::TcpListener, sync::mpsc},
    tracing,
};
use kube::ServiceAccountToken;
use pod_exec::{
    connector::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl},
    msg_handle::handle_websocket,
};

pub async fn init_router() {
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

    let (tx_web, mut rx_web) = mpsc::channel::<Message>(100);
    let (tx_kube, mut rx_kube) = mpsc::channel(100);

    let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
    match conn {
        Ok(mut ws_stream) => {
            let mut closed = false;
            tokio::spawn(async move {
                handle_websocket(&mut ws_stream, &mut rx_web, &tx_kube, &mut closed, None).await;
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

                if tx_web.send(client_msg).await.is_err() {
                    tracing::info!("Failed to send message to channel");
                }
            },
            Some(kube_msg) = rx_kube.recv() => {
                tracing::debug!("Received from kubernetes: {}", kube_msg);
                let kube_msg = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, kube_msg);
                let kube_msg = Message::Text(format!("1{}", kube_msg));
                if socket.send(kube_msg).await.is_err() {
                    tracing::info!("Client disconnected, failed to send message");
                }
            }
        }
    }
}
