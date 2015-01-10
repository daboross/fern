use std::io;
use std::io::stdio;

use api;
use loggers;

/// This is the base logger configuration in fern.
///
/// All LoggerConfig will do is filter log messages based on level, and then pass on to any number
/// of other loggers.
#[unstable]
pub struct LoggerConfig {
    /// The format for this logger. All log messages coming in will be sent through this closure
    /// before being sent to parent loggers
    pub format: Box<Fn(&str, &api::Level) -> String + Sync + Send>,
    /// A list of loggers to send messages to. Any messages that are sent to this logger that
    /// aren't filtered are sent to each of these loggers in turn.
    pub output: Vec<OutputConfig>,
    /// The level of this logger. Any messages which have a lower level than this level won't be
    /// passed on.
    pub level: api::Level,
}

/// This enum contains various outputs that you can send messages to.
///
/// You can use this in conjunction with LoggerConfig for message formating and filtering, or just
/// use this if you don't need to filter or format messages.
#[experimental]
pub enum OutputConfig {
    /// Parent logger - another LoggerConfig
    #[unstable]
    Parent(LoggerConfig),
    /// File logger - all messages sent to this will be output into the specified path.
    /// Note that the file will be opened appending, so nothing in the file will be overwritten.
    File(Path),
    /// Stdout logger - all messages sent to this will be printed to stdout
    Stdout,
    /// Stderr logger - all messages sent to this will be printed to stderr
    Stderr,
    /// Null logger - all messages sent to this logger will simply disappear into the void
    Null,
    /// Custom logger - all messages sent here will just be sent on to the logger implementation
    /// you provide
    Custom(api::BoxedLogger),
}

#[experimental]
impl OutputConfig {
    /// Builds this OutputConfig into an actual Logger that you can send messages to. This will
    /// open any files, get handles to stdout/stderr, etc. depending on which type of logger this
    /// is.
    #[unstable]
    pub fn into_logger(self) -> io::IoResult<api::BoxedLogger> {
        return Ok(match self {
            OutputConfig::Parent(config) => try!(config.into_logger()),
            OutputConfig::File(ref path) => Box::new(try!(
                loggers::WriterLogger::<io::File>::with_file(path))) as api::BoxedLogger,
            OutputConfig::Stdout => Box::new(
                loggers::WriterLogger::<stdio::StdWriter>::with_stdout()) as api::BoxedLogger,
            OutputConfig::Stderr => Box::new(
                loggers::WriterLogger::<stdio::StdWriter>::with_stderr()) as api::BoxedLogger,
            OutputConfig::Null => Box::new(loggers::NullLogger) as api::BoxedLogger,
            OutputConfig::Custom(log) => log,
        });
    }
}

#[experimental]
impl LoggerConfig {
    /// Builds this LoggerConfig into an actual Logger that you can send messages to. This will
    /// build all parent OutputConfig loggers as well.
    #[unstable]
    pub fn into_logger(self) -> io::IoResult<api::BoxedLogger> {
        let LoggerConfig {format, level, output} = self;
        let log = try!(loggers::ConfigurationLogger::new(format, output, level));
        return Ok(Box::new(log) as api::BoxedLogger);
    }
}
