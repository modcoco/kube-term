use axum::{routing::get, Router};
use common::{
    axum::{
        self,
        routing::{on, MethodFilter},
    },
    tokio::net::TcpListener,
    tracing,
};
use pod_exec::handler;

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
