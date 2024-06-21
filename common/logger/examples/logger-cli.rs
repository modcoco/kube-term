use log::info;

fn main() {
    logger::logger_trace::setup_logger();
    info!("Client test")
}
