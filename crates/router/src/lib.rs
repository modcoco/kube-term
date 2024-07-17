use std::borrow::Cow;

use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use common::{
    axum::{
        self,
        extract::{ws::Message, RawPathParams},
        routing::{on, MethodFilter},
    },
    tokio::{self, net::TcpListener, sync::mpsc},
    tracing,
};
use kube::ServiceAccountToken;
use pod_exec::{
    connector::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl},
    msg_handle::handle_websocket,
};

// todo post update to ws
pub async fn init_router() {
    let app = Router::new()
        .route("/health", get(|| async { "Hello, World!" }))
        .route(
            "/namespace/:namespace/pod/:pod/container/:container",
            on(MethodFilter::GET, handler),
        );

    tracing::info!("start web server...");
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Default)]
struct ContainerCoords {
    namespace: String,
    pod: String,
    container: String,
}

impl ContainerCoords {
    fn populate_from_raw_path_params(mut self, raw_path_params: &RawPathParams) -> Self {
        for (key, value) in raw_path_params.iter() {
            match key {
                "namespace" => value.clone_into(&mut self.namespace),
                "pod" => value.clone_into(&mut self.pod),
                "container" => value.clone_into(&mut self.container),
                _ => todo!(),
            }
        }
        self
    }
}

async fn handler(ws: WebSocketUpgrade, raw_path_params: RawPathParams) -> Response {
    let coords = ContainerCoords::default().populate_from_raw_path_params(&raw_path_params);
    tracing::info!("{:?}", coords);

    let protocols: Vec<Cow<'static, str>> = vec![Cow::Borrowed("echo-protocol")];
    ws.protocols(protocols)
        .on_upgrade(|axum_socket| handle_socket(axum_socket, coords))
}

async fn handle_socket(mut axum_socket: WebSocket, coords: ContainerCoords) {
    let sat = ServiceAccountToken::new();
    let pod_exec_url = PodExecUrl {
        domain: String::from(&sat.kube_host),
        port: String::from(&sat.kube_port),
        path: PodExecPath {
            base_path: String::from("/api/v1"),
            namespace: String::from("/namespaces/default"),
            pod: String::from("/pods/web-term-559fdfcd89-sck7p"),
            tail_path: String::from("/exec"),
        },
    };
    let pod_exec_params = PodExecParams {
        container: coords.container,
        stdin: true,
        stdout: true,
        stderr: true,
        tty: true,
        command: "env&env=TERM%3Dxterm&command=COLUMNS%3D800&command=LINES%3D10&command=bash"
            .to_string(),
        pretty: true,
        follow: true,
    };

    let (tx_web, mut rx_web) = mpsc::channel::<Message>(100);
    let (tx_kube, mut rx_kube) = mpsc::channel(100);

    let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
    match conn {
        Ok(mut kube_ws_stream) => {
            let mut closed = false;
            tokio::spawn(async move {
                handle_websocket(
                    &mut kube_ws_stream,
                    &mut rx_web,
                    &tx_kube,
                    &mut closed,
                    None,
                )
                .await;
            });
        }
        Err(err) => {
            tracing::error!("ERROR, {}", err)
        }
    };

    loop {
        tokio::select! {
            Some(client_msg) = axum_socket.recv() => {
                let client_msg = if let Ok(client_msg) = client_msg {
                    tracing::debug!("Received from client: {:?}", client_msg);
                    client_msg
                } else {
                    tracing::info!("Client disconnected, the msg isn't ok");
                    return;
                };

                if tx_web.send(client_msg).await.is_err() {
                    tracing::info!("Failed to send message to channel");
                }
            },
            Some(kube_msg) = rx_kube.recv() => {
                tracing::debug!("Received from kubernetes: {}", kube_msg);
                let kube_msg = Message::Text(kube_msg);
                if axum_socket.send(kube_msg).await.is_err() {
                    tracing::info!("Client disconnected, failed to send message");
                }
            }
        }
    }
}
