//! Tests!
#[cfg(feature = "manual")]
use std::{fs, io::prelude::*};

#[cfg(feature = "manual")]
use log::Level::*;

mod support;

#[cfg(feature = "manual")]
use support::manual_log;

#[cfg(feature = "manual")]
#[test]
fn test_basic_logging_manual_rotating() {
    // Create a basic logger configuration
    let (_max_level, logger) = fern::Dispatch::new()
        .format(|out, msg, record| out.finish(format_args!("[{}] {}", record.level(), msg)))
        .level(log::LevelFilter::Info)
        .chain(fern::Manual::new("program.log.", "%Y-%m-%d"))
        .into_log();

    let l = &*logger;
    manual_log(l, Trace, "SHOULD NOT DISPLAY");
    manual_log(l, Debug, "SHOULD NOT DISPLAY");
    manual_log(l, Info, "Test information message");
    manual_log(l, Warn, "Test warning message");
    manual_log(l, Error, "Test error message");

    // ensure all File objects are dropped and OS buffers are flushed.
    log::logger().flush();
}
