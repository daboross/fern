//! Tests!
extern crate fern;
#[macro_use]
extern crate log;
extern crate tempdir;

use std::sync::{Arc, Mutex};
use std::io::prelude::*;
use std::{fmt, fs, io};

use log::Level::*;

#[test]
fn test_global_logger() {
    // custom logger to verify behavior
    struct LogVerify {
        info: bool,
        warn: bool,
        error: bool,
    }
    impl LogVerify {
        fn new() -> Self {
            LogVerify {
                info: false,
                warn: false,
                error: false,
            }
        }
        fn log(&mut self, record: &log::Record) {
            let formatted_message = format!("{}", record.args());
            match &*formatted_message {
                "[INFO] Test information message" => {
                    assert_eq!(self.info, false, "expected only one info message");
                    self.info = true;
                }
                "[WARN] Test warning message" => {
                    assert_eq!(self.warn, false, "expected only one warn message");
                    self.warn = true;
                }
                "[ERROR] Test error message" => {
                    assert_eq!(self.error, false, "expected only one error message");
                    self.error = true;
                }
                other => panic!("unexpected message: '{}'", other),
            }
        }
    }
    #[derive(Clone)]
    struct LogVerifyWrapper(Arc<Mutex<LogVerify>>);
    impl log::Log for LogVerifyWrapper {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn flush(&self) {}
        fn log(&self, record: &log::Record) {
            self.0.lock().unwrap().log(record);
        }
    }

    let verify = LogVerifyWrapper(Arc::new(Mutex::new(LogVerify::new())));

    // Create a basic logger configuration
    fern::Dispatch::new()
        .format(|out, msg, record| {
            out.finish(format_args!("[{}] {}", record.level(), msg))
        })
        // Only log messages Info and above
        .level(log::LevelFilter::Info)
        // Output to our verification logger for verification
        .chain(Box::new(verify.clone()) as Box<log::Log>)
        .apply()
        .expect("Failed to initialize logger: global logger already set!");

    trace!("SHOULD NOT DISPLAY");
    debug!("SHOULD NOT DISPLAY");
    info!("Test information message");
    warn!("Test warning message");
    error!("Test error message");

    // ensure all buffers are flushed.
    log::logger().flush();

    let verify_acquired = verify.0.lock().unwrap();
    assert_eq!(
        verify_acquired.info,
        true,
        "expected info message to be received"
    );
    assert_eq!(
        verify_acquired.warn,
        true,
        "expected warn message to be received"
    );
    assert_eq!(
        verify_acquired.error,
        true,
        "expected error message to be received"
    );
}

#[test]
fn basic_logging_file_logging() {
    // Create a temporary directory to put a log file into for testing
    let temp_log_dir = tempdir::TempDir::new("fern").expect("Failed to set up temporary directory");
    let log_file = temp_log_dir.path().join("test.log");

    // Create a basic logger configuration
    let l = fern::Dispatch::new()
        .format(|out, msg, record| {
            out.finish(format_args!("[{}] {}", record.level(), msg))
        })
        // Only log messages Info and above
        .level(log::LevelFilter::Info)
        // Output to stdout and the log file in the temporary directory we made above to test
        .chain(io::stdout())
        .chain(fern::log_file(log_file).expect("Failed to open log file"))
        .into_log().1;

    manual_log(&*l, Trace, "SHOULD NOT DISPLAY");
    manual_log(&*l, Debug, "SHOULD NOT DISPLAY");
    manual_log(&*l, Info, "Test information message");
    manual_log(&*l, Warn, "Test warning message");
    manual_log(&*l, Error, "Test error message");

    // ensure all File objects are dropped and OS buffers are flushed.
    log::logger().flush();

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

    drop(l); // close before tempdir closes for Windows support.

    temp_log_dir
        .close()
        .expect("Failed to clean up temporary directory");
}

/// Utility to manually enter a log message into a logger. All extra metadata (target,
/// line number, etc) will be blank.
fn manual_log<T>(logger: &log::Log, level: log::Level, message: T)
where
    T: fmt::Display,
{
    logger.log(&log::RecordBuilder::new()
        .args(format_args!("{}", message))
        .level(level)
        .build());
}

#[test]
fn test2_line_seps() {
    // Create a temporary directory to put a log file into for testing
    let temp_log_dir = tempdir::TempDir::new("fern").expect("Failed to set up temporary directory");
    let log_file = temp_log_dir.path().join("test_custom_line_sep.log");

    // Create a basic logger configuration
    let (_, l) = fern::Dispatch::new()
        // default format is just the message if not specified
        // default log level is 'trace' if not specified (logs all messages)
        // output to the log file with the "\r\n" line separator.
        .chain(fern::Output::file(fern::log_file(&log_file).expect("Failed to open log file"), "\r\n"))
        .into_log();

    manual_log(&*l, Info, "message1");
    manual_log(&*l, Info, "message2");

    // ensure all File objects are dropped and OS buffers are flushed.
    l.flush();

    {
        let result = {
            let mut log_read = fs::File::open(&temp_log_dir.path().join("test_custom_line_sep.log")).unwrap();
            let mut buf = String::new();
            log_read.read_to_string(&mut buf).unwrap();
            buf
        };
        assert_eq!(&result, "message1\r\nmessage2\r\n");
    }

    temp_log_dir
        .close()
        .expect("Failed to clean up temporary directory");
}

#[test]
fn test3_channel_logging() {
    use std::sync::mpsc;
    // Create the channel
    let (send, recv) = mpsc::channel();

    let (_, l) = fern::Dispatch::new().chain(send).into_log();

    manual_log(&*l, Info, "message1");
    manual_log(&*l, Info, "message2");

    l.flush();

    assert_eq!(recv.recv().unwrap(), "message1\n");
    assert_eq!(recv.recv().unwrap(), "message2\n");
}
