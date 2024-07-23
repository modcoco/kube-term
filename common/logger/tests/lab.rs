#[test]
fn logs_file() {
    let (_handle, _guard) = logger::logger_trace::init_logger("test_boot", true);
    tracing::info!("Client test");
}
