use chrono::Local;
use std::sync::Arc;
use tokio::time::Instant;
use tracing::{info, Level};
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

pub fn setup_logger() -> Arc<tokio::time::Instant> {
    let start_time = Instant::now();
    let start_time: Arc<Instant> = Arc::new(start_time);

    pub struct LocalTimer;
    impl FormatTime for LocalTimer {
        fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
            write!(w, "{}", Local::now().format("%FT%T%.3f"))
        }
    }

    let format = tracing_subscriber::fmt::format()
        .with_line_number(true)
        .with_level(true)
        .with_target(true)
        .with_timer(LocalTimer);

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        // .with_writer(io::stdout) // 写入标准输出
        // .with_writer(non_blocking)
        .with_ansi(true) // 如果日志是写入文件，应将ansi的颜色输出功能关掉
        .event_format(format)
        .init();

    info!("init");

    start_time
}

#[allow(dead_code)]
#[test]
fn main() {
    setup_logger();
    tracing::info!("This is not an example");
}
