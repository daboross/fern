extern crate fern;
extern crate log;
extern crate time;

#[test]
fn test() {
    // As a workaround for https://github.com/rust-lang/cargo/issues/1474, this test is copied
    // from lib.rs docs
    let logger_config = fern::DispatchConfig {
        format: Box::new(|msg: &str, level: &log::LogLevel, _location: &log::LogLocation| {
            // This is a fairly simple format, though it's possible to do more complicated ones.
            // This closure can contain any code, as long as it produces a String message.
            format!("[{}][{}] {}", time::now().strftime("%Y-%m-%d][%H:%M:%S").unwrap(), level, msg)
        }),
        output: vec![fern::OutputConfig::stdout(), fern::OutputConfig::file("output.log")],
        level: log::LogLevelFilter::Trace,
    };
    // So this isn't considered unused without changing any of the actual test text copied directly
    // from docs.
    drop(logger_config);
}
