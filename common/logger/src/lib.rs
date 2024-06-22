pub mod logger_env;
pub mod logger_trace;

pub use logger_trace as other_logger_trace;

// pub use logger_env;
// pub use logger_trace;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {

    use log::info;

    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn log_trace() {
        logger_trace::setup_logger();
        info!("Test trace log.")
    }

    #[test]
    fn log_env() {
        logger_env::setup_logger(true, Some(""));
        info!("Test env log.")
    }
}
