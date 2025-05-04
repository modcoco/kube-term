use common::axum::{self};
use common::dotenv;
use common::{
    tokio::{self, net::TcpListener},
    tracing,
};
use logger::logger_trace::init_logger;
use router::init_router;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let (_h, _w) = init_logger("pod-exec", true);
    let router = init_router().await;
    tracing::info!("start web server...");
    let listener = TcpListener::bind("0.0.0.0:8081").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
