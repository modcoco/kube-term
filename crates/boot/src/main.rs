use common::tokio;
use logger::logger_trace::init_logger;
use router::init_router;

#[tokio::main]
async fn main() {
    init_logger();
    init_router().await;
}
