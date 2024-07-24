use axum::extract::Extension;
use axum::extract::Query;
use common::axum;
use common::chrono::Local;
use common::tokio::{self, time::Instant};
use common::tracing_appender;
use common::tracing_appender::non_blocking::WorkerGuard;
use common::tracing_appender::rolling::RollingFileAppender;
use common::tracing_appender::rolling::Rotation;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::EnvFilter;

pub type ReloadLogLevelHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, tracing_subscriber::Registry>;

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

    start_time
}

pub struct LocalTimer;
impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%FT%T%.3f"))
    }
}

pub fn init_logger(
    app_name: &str,
    log_to_file: bool,
) -> (ReloadLogLevelHandle, Option<WorkerGuard>) {
    let default_filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
    );
    let (filter, reload_handle) = tracing_subscriber::reload::Layer::new(default_filter);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_level(true)
        .with_target(true)
        .with_ansi(true)
        .with_timer(LocalTimer);

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer);

    let guard = if log_to_file {
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            get_os_log_directory(app_name),
            to_snake_case(app_name),
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = tracing_subscriber::fmt::layer()
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_level(true)
            .with_target(true)
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_timer(LocalTimer);
        _ = registry.with(file_layer).try_init();
        Some(guard)
    } else {
        _ = registry.try_init();
        None
    };

    tracing::info!("Logger initialized");
    (reload_handle, guard)
}

#[allow(dead_code)]
#[test]
fn main() {
    let (_handle, _guard) = init_logger("test_boot", false);

    tracing::info!("Client test");

    // drop(guard);
    tracing::info!("This is not an example");
}

// WEB
#[derive(serde::Deserialize)]
pub struct LevelFlag {
    level: String,
}
pub async fn change_log_level(
    Query(flag): Query<LevelFlag>,
    Extension(reload_handle): Extension<ReloadLogLevelHandle>,
) -> String {
    match flag.level.parse::<EnvFilter>() {
        Ok(env_filter) => {
            reload_handle.modify(|filter| *filter = env_filter).unwrap();
            "ok".to_string()
        }
        Err(err) => err.to_string(),
    }
}

// CMD
// export RUST_LOG=debug
// let reload_handle = logger::logger_trace::init_logger();
// std::env::set_var("RUST_LOG", "info");
// let new_filter = tracing_subscriber::EnvFilter::from_default_env();
// _ = reload_handle.reload(new_filter);

#[cfg(target_os = "linux")]
fn is_root() -> bool {
    unsafe { common::libc::getuid() == 0 }
}

#[cfg(target_os = "windows")]
fn is_root() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn is_root() -> bool {
    true
}

pub fn get_os_log_directory(app_name: &str) -> PathBuf {
    let _snake_name = to_snake_case(app_name);
    let _camel_case_name = to_camel_case(app_name);

    if !is_root() {
        return PathBuf::from("logs");
    }

    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("/var/log/{}", _snake_name))
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(format!("C:\\ProgramData\\{}\\Logs", _camel_case_name))
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from(format!("/Library/Logs/{}", _camel_case_name))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        PathBuf::from("logs")
    }
}

pub fn to_snake_case(input: &str) -> String {
    let mut result = String::new();
    let mut prev_char_was_upper = false;

    for (i, c) in input.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_char_was_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
            prev_char_was_upper = true;
        } else {
            result.push(c);
            prev_char_was_upper = false;
        }
    }

    result.to_lowercase().replace(' ', "")
}

pub fn to_camel_case(input: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in input.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                result.push(c.to_uppercase().next().unwrap_or(c));
                capitalize_next = false;
            } else {
                result.push(c);
            }
        } else {
            capitalize_next = true;
        }
    }

    result
}
