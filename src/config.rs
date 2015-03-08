use std::io;
use std::fs;
use std::path;

use api;
use loggers;

/// This is the base logger configuration in fern.
///
/// All DispatchConfig will do is filter log messages based on level, and then pass on to any
/// number of other loggers.
#[unstable]
pub struct DispatchConfig {
    /// The format for this logger. All log messages coming in will be sent through this closure
    /// before being sent to parent loggers
    pub format: Box<Formatter>,
    /// A list of loggers to send messages to. Any messages that are sent to this logger that
    /// aren't filtered are sent to each of these loggers in turn.
    pub output: Vec<OutputConfig>,
    /// The level of this logger. Any messages which have a lower level than this level won't be
    /// passed on.
    pub level: api::Level,
}

pub type Formatter = Fn(&str, &api::Level) -> String + Sync + Send;

/// This enum contains various outputs that you can send messages to.
///
/// You can use this in conjunction with DispatchConfig for message formating and filtering, or
/// just use this if you don't need to filter or format messages.
#[unstable]
pub enum OutputConfig {
    /// Child logger - another DispatchConfig
    Child(DispatchConfig),
    /// File logger - all messages sent to this will be output into the specified path. Note that
    /// the file will be opened appending, so nothing in the file will be overwritten.
    File(path::PathBuf),
    /// Stdout logger - all messages sent to this will be printed to stdout.
    Stdout,
    /// Stderr logger - all messages sent to this will be printed to stderr.
    Stderr,
    /// Null logger - all messages sent to this logger will simply disappear into the void.
    Null,
    /// Custom logger - all messages sent here will just be sent on to the logger implementation
    /// you provide.
    Custom(Box<api::Logger>),
}

#[unstable]
impl IntoLog for OutputConfig {
    fn into_logger(self) -> io::Result<Box<api::Logger>> {
        return Ok(match self {
            OutputConfig::Child(config) => try!(config.into_logger()),
            OutputConfig::File(ref path) => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file(path))) as Box<api::Logger>,
            OutputConfig::Stdout => Box::new(
                loggers::WriterLogger::<io::Stdout>::with_stdout()) as Box<api::Logger>,
            OutputConfig::Stderr => Box::new(
                loggers::WriterLogger::<io::Stderr>::with_stderr()) as Box<api::Logger>,
            OutputConfig::Null => Box::new(loggers::NullLogger) as Box<api::Logger>,
            OutputConfig::Custom(log) => log,
        });
    }
}

#[unstable]
impl IntoLog for DispatchConfig {
    fn into_logger(self) -> io::Result<Box<api::Logger>> {
        let DispatchConfig {format, level, output} = self;
        let log = try!(loggers::DispatchLogger::new(format, output, level));
        return Ok(Box::new(log) as Box<api::Logger>);
    }
}

#[unstable]
/// Trait which represents any logger configuration which can be built into a `fern::Logger`.
pub trait IntoLog {
    /// Builds this config into an actual Logger that you can send messages to. This will
    /// open any files, get handles to stdout/stderr, etc. depending on which type of logger this
    /// is.
    fn into_logger(self) -> io::Result<Box<api::Logger>>;
}
