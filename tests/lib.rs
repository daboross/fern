//! Tests!
//!
//! Note that since all tests set the global logger, only one can be run at a time in each runtime. In order to
//! successfully run tests, you can use the following:
//!
//! ```sh
//! cargo test -- --exclude test2
//! cargo test test2
//! ```
extern crate fern;
#[macro_use]
extern crate log;
extern crate tempdir;

use std::io::prelude::*;
use std::{fs, io};

#[test]
fn test1_basic_usage() {
    // Create a temporary directory to put a log file into for testing
    let temp_log_dir = tempdir::TempDir::new("fern").expect("Failed to set up temporary directory");
    let log_file = temp_log_dir.path().join("test.log");

    // Create a basic logger configuration
    fern::Dispatch::new()
        .format(|out, msg, record| {
            out.finish(format_args!("[{}] {}", record.level(), msg))
        })
        // Only log messages Info and above
        .level(log::LogLevelFilter::Info)
        // Output to stdout and the log file in the temporary directory we made above to test
        .chain(io::stdout())
        .chain(fern::log_file(log_file).expect("Failed to open log file"))
        .apply()
        .expect("Failed to initialize logger: global logger already set!");

    trace!("SHOULD NOT DISPLAY");
    debug!("SHOULD NOT DISPLAY");
    info!("Test information message");
    warn!("Test warning message");
    error!("Test error message");

    // shutdown the logger, to ensure all File objects are dropped and OS buffers are flushed.
    log::shutdown_logger().expect("Failed to shutdown logger");

    {
        let result = {
            let mut log_read = fs::File::open(&temp_log_dir.path().join("test.log")).unwrap();
            let mut buf = String::new();
            log_read.read_to_string(&mut buf).unwrap();
            buf
        };
        assert!(
            !result.contains("SHOULD NOT DISPLAY"),
            "expected result not including \"SHOULD_NOT_DISPLAY\", found:\n```\n{}\n```\n",
            result
        );
        assert!(
            result.contains("[INFO] Test information message"),
            "expected result including \"[INFO] Test information message\", found:\n```\n{}\n```\n",
            result
        );
        assert!(
            result.contains("[WARN] Test warning message"),
            "expected result including \"[WARN] Test warning message\", found:\n```\n{}\n```\n",
            result
        );
        assert!(
            result.contains("[ERROR] Test error message"),
            "expected result to not include \"[ERROR] Test error message\", found:\n```\n{}\n```\n",
            result
        );
    }

    // Just to make sure this goes smoothly - it dose this automatically if we don't .close()
    // manually, but it will ignore any errors when doing it automatically.
    temp_log_dir
        .close()
        .expect("Failed to clean up temporary directory");
}

#[test]
fn test2_line_seps() {
    // Create a temporary directory to put a log file into for testing
    let temp_log_dir = tempdir::TempDir::new("fern").expect("Failed to set up temporary directory");
    let log_file = temp_log_dir.path().join("test_custom_line_sep.log");

    // Create a basic logger configuration
    fern::Dispatch::new()
        // default format is just the message if not specified
        // default log level is 'trace' if not specified (logs all messages)
        // output to the log file with the "\r\n" line separator.
        .chain(fern::Output::file(fern::log_file(&log_file).expect("Failed to open log file"), "\r\n"))
        .apply()
        .expect("Failed to initialize logger: global logger already set!");

    info!("message1");
    info!("message2");

    // shutdown the logger, to ensure all File objects are dropped and OS buffers are flushed.
    log::shutdown_logger().expect("Failed to shutdown logger");

    {
        let result = {
            let mut log_read = fs::File::open(&temp_log_dir.path().join("test_custom_line_sep.log")).unwrap();
            let mut buf = String::new();
            log_read.read_to_string(&mut buf).unwrap();
            buf
        };
        assert_eq!(&result, "message1\r\nmessage2\r\n");
    }

    // Just to make sure this goes smoothly - it dose this automatically if we don't .close()
    // manually, but it will ignore any errors when doing it automatically.
    temp_log_dir
        .close()
        .expect("Failed to clean up temporary directory");
}

#[test]
fn test3_channel_logging() {
    use std::sync::mpsc;
    // Create the channel
    let (send, recv) = mpsc::channel();

    fern::Dispatch::new()
        .chain(send)
        .apply()
        .expect("Failed to initialize logger: global logger already set!");

    info!("message1");
    info!("message2");

    log::shutdown_logger().expect("Failed to shutdown logger");

    assert_eq!(recv.recv().unwrap(), "message1\n");
    assert_eq!(recv.recv().unwrap(), "message2\n");
}
