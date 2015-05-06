#[macro_use]
extern crate log;
extern crate fern;
extern crate tempdir;

use std::io::prelude::*;
use std::fs;

mod doc_test_copy;

#[test]
fn basic_usage_test() {
    // Create a temporary directory to put a log file into for testing
    let temp_log_dir = tempdir::TempDir::new("fern").ok()
                        .expect("Failed to set up temporary directory");
    let log_file = temp_log_dir.path().join("test.log");

    // Create a basic logger configuration
    let logger_config = fern::DispatchConfig {
        format: Box::new(|msg, level, _location| {
            // This format just displays [{level}] {message}
            format!("[{}] {}", level, msg)
        }),
        // Output to stdout and the log file in the temporary directory we made above to test
        output: vec![fern::OutputConfig::stdout(), fern::OutputConfig::file(&log_file)],
        // Only log messages Info and above
        level: log::LogLevelFilter::Info,
    };

    if let Err(e) = fern::init_global_logger(logger_config, log::LogLevelFilter::Trace) {
        panic!("Failed to initialize global logger: {}", e);
    }

    trace!("SHOULD NOT DISPLAY");
    debug!("SHOULD NOT DISPLAY");
    info!("Test information message");
    warn!("Test warning message");
    error!("Test error message");

    {
        let result = {
            let mut log_read = fs::File::open(&temp_log_dir.path().join("test.log")).unwrap();
            let mut buf = String::new();
            log_read.read_to_string(&mut buf).unwrap();
            buf
        };
        assert!(!result.contains("SHOULD NOT DISPLAY"));
        assert!(result.contains("[INFO] Test information message"));
        assert!(result.contains("[WARN] Test warning message"));
        assert!(result.contains("[ERROR] Test error message"));
    }

    // Just to make sure this goes smoothly - it dose this automatically if we don't .close()
    // manually, but it will ignore any errors when doing it automatically.
    temp_log_dir.close().ok().expect("Failed to clean up temporary directory");
}

#[test]
fn custom_line_sep_test() {
    // Create a temporary directory to put a log file into for testing
    let temp_log_dir = tempdir::TempDir::new("fern").ok()
                        .expect("Failed to set up temporary directory");
    let log_file = temp_log_dir.path().join("test_custom_line_sep.log");

    // This is done in a new scope, because why not.
    {
        // Create a basic logger configuration
        let logger_config = fern::DispatchConfig {
            format: Box::new(|msg, _level, _location| {
                // This format just displays {message}
                msg.to_string()
            }),
            // Output to stdout and the log file in the temporary directory we made above to test
            output: vec![fern::OutputConfig::file_with_line_sep(&log_file, "\r\n")],
            // Log all messages
            level: log::LogLevelFilter::Trace,
        };
        // we can't init global logger for this test as it is already initialized in the general
        // usage test.
        let fern_logger = fern::IntoLog::into_fern_logger(logger_config).unwrap();

        let location = construct_fake_log_location();
        let level = log::LogLevel::Info;

        fern_logger.log("message1", &level, &location).unwrap();
        fern_logger.log("message2", &level, &location).unwrap();
    }
    {
        let result = {
            let mut log_read = fs::File::open(
                &temp_log_dir.path().join("test_custom_line_sep.log")).unwrap();
            let mut buf = String::new();
            log_read.read_to_string(&mut buf).unwrap();
            buf
        };
        assert_eq!(&result, "message1\r\nmessage2\r\n");
    }

    // Just to make sure this goes smoothly - it dose this automatically if we don't .close()
    // manually, but it will ignore any errors when doing it automatically.
    temp_log_dir.close().ok().expect("Failed to clean up temporary directory");
}

/// This may be a bad idea, but it seems necessary for using fern::Logger. Maybe this should be
/// fixed. This may break with upstream changes from log, but since this is only in tests, not in
/// the actual code, it should be fine.
fn construct_fake_log_location() -> log::LogLocation {
    return log::LogLocation {
        __module_path: "test",
        __file: "tests.rs",
        __line: 0,
    };
}
