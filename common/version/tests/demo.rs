pub fn add(left: usize, right: usize) -> usize {
    left + right
}

fn to_gb(value: i64) -> i64 {
    ((value as f64) / (1000.0 * 1000.0)).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn str_trimmed() {
        let str = "nvidia.com";
        let trimmed_str = str.trim_end_matches(".com");
        println!("{}", trimmed_str);
    }

    #[test]
    fn calculate() {
        let memory: i64 = 1055924872;
        let memory = ((memory as f64) / (1000.0 * 1000.0)).round() as i64;
        println!("{}", memory);

        let memory: i64 = 1055924872;
        let memory = memory / (1000 * 1000);
        println!("{}", memory);

        let memory: i64 = 1055924872;
        let memory = to_gb(memory);
        println!("{}", memory);
    }

    fn naive_datetime_with_offset(start_time: NaiveDateTime, offset_hours: i32) -> NaiveDateTime {
        let start_time_utc: DateTime<Utc> =
            DateTime::<Utc>::from_naive_utc_and_offset(start_time, Utc);
        let offset = FixedOffset::east_opt(offset_hours * 3600).expect("Failed to create offset");
        let start_time_with_offset = start_time_utc.with_timezone(&offset);

        start_time_with_offset.naive_local()
    }

    #[test]
    fn data_without_zone() {
        let start_time = NaiveDateTime::parse_from_str("2024-07-08 12:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse date");

        let naive_start_time_utc_plus_8 = naive_datetime_with_offset(start_time, 8);

        println!("NaiveDateTime with UTC+8: {}", naive_start_time_utc_plus_8);
    }

    #[test]
    fn logs_file() {
        let appender = tracing_appender::rolling::never(".", "cluster.log");
        let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender);
        tracing_subscriber::fmt()
            .with_writer(non_blocking_appender)
            .with_ansi(false)
            .init();
        tracing::info!("test")
    }

    #[test]
    fn remove_utc() {
        let start_time: NaiveDateTime =
            chrono::NaiveDateTime::parse_from_str("2024-07-16 17:44:14", "%Y-%m-%d %H:%M:%S")
                .map_err(|_| sqlx::Error::Configuration("invalid start_time format".into()))
                .unwrap();

        let start_time = start_time - chrono::Duration::hours(8);
        println!("{}", start_time);
        let now = chrono::Local::now().naive_local() - chrono::Duration::hours(8);

        println!("{}", now);
    }
}
