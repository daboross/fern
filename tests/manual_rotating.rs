//! Tests!
#[cfg(feature = "manual")]
use std::fs;

mod support;

#[cfg(feature = "manual")]
#[test]
fn test_basic_logging_manual_rotating() {
    // Create a basic logger configuration
    let (level, dispatch) = fern::Dispatch::new()
        .format(|out, msg, record| out.finish(format_args!("[{}] {}", record.level(), msg)))
        .level(log::LevelFilter::Info)
        .chain(fern::Manual::new("program.log.", "%Y-%m-%d_%H-%M-%S%.f"))
        .into_dispatch_with_arc();

    if level == log::LevelFilter::Off {
        log::set_boxed_logger(Box::new(NullLogger)).unwrap();
    } else {
        log::set_boxed_logger(Box::new(dispatch.clone())).unwrap();
    }
    log::set_max_level(level);

    log::trace!("SHOULD NOT DISPLAY");
    log::debug!("SHOULD NOT DISPLAY");
    log::info!("Test information message");
    log::warn!("Test warning message");
    log::error!("Test error message");

    let res = dispatch.rotate();

    log::trace!("SHOULD NOT DISPLAY");
    log::debug!("SHOULD NOT DISPLAY");
    log::info!("Test information message");
    log::warn!("Test warning message");
    log::error!("Test error message");

    for output in res.iter() {
        match output {
            Some((old_path, new_path)) => {
                log::info!("old path: {}", fs::canonicalize(old_path).unwrap().to_string_lossy());
                log::info!("new path: {}", fs::canonicalize(new_path).unwrap().to_string_lossy());
            }
            None => {}
        }
    }

    // ensure all File objects are dropped and OS buffers are flushed.
    log::logger().flush();
}

struct NullLogger;

impl log::Log for NullLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _: &log::Record) {}

    fn flush(&self) {}
}
