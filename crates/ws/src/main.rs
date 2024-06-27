use common::tokio::net::TcpStream;
use common::tokio::sync::mpsc;
use common::{tokio, tokio_tungstenite, tracing};
use kube::ServiceAccountToken;
use logger::logger_trace::init_logger;
use msg_handle::{handle_websocket, stdin_reader};
use pod_exec::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

mod msg_handle;
mod pod_exec;

#[tokio::main]
async fn main() {
    init_logger();
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

    let (tx_cmd, mut rx_cmd) = mpsc::channel(100);
    let (tx_ws, mut rx_ws) = mpsc::channel(100);

    stdin_reader(tx_cmd).await;

    let conn: Result<WebSocketStream<MaybeTlsStream<TcpStream>>, common::anyhow::Error> =
        pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
    match conn {
        Ok(mut ws_stream) => {
            let mut closed = false;
            tokio::spawn(async move {
                handle_websocket(&mut ws_stream, &mut rx_cmd, &tx_ws, &mut closed).await;
            });
        }
        Err(err) => {
            tracing::error!("ERROR, {}", err)
        }
    };
    while let Some(msg) = rx_ws.recv().await {
        tracing::info!("Received from ws: {}", msg);
    }
}
