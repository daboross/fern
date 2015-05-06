use std::convert::AsRef;
use std::io;
use std::fs;
use std::path;

use log;

use api;
use loggers;
use errors::InitError;

/// This is the base logger configuration in fern.
///
/// All DispatchConfig will do is filter log messages based on level, pass the message through the
/// Formatter, and then pass on to any number of output loggers.
pub struct DispatchConfig<'a> {
    /// The format for this logger. All log messages coming in will be sent through this closure
    /// before being sent to child loggers.
    pub format: Box<Formatter>,
    /// A list of loggers to send messages to. Any messages that are sent to this logger that
    /// aren't filtered are sent to each of these loggers in turn.
    pub output: Vec<OutputConfig<'a>>,
    /// The level of this logger. Any messages which have a lower level than this level won't be
    /// passed on.
    pub level: log::LogLevelFilter,
}

pub type Formatter = Fn(&str, &log::LogLevel, &log::LogLocation) -> String + Sync + Send;


/// This enum contains various outputs that you can send messages to.
enum OutputConfigOptions<'a> {
    /// Child logger - send messages to another DispatchConfig.
    Child(DispatchConfig<'a>),
    /// File logger - all messages sent to this will be output into the specified path. Note that
    /// the file will be opened appending, so nothing in the file will be overwritten.
    File { path: &'a path::Path, line_sep: &'a str },
    /// File logger with OpenOptions - all messages will be sent to the specified file. The file
    /// will be opened using the specified OpenOptions.
    FileOptions { path: &'a path::Path, options: &'a fs::OpenOptions, line_sep: &'a str },
    /// Stdout logger - all messages sent to this will be printed to stdout.
    Stdout,
    /// Stderr logger - all messages sent to this will be printed to stderr.
    Stderr,
    /// Null logger - all messages sent to this logger will disappear into the void.
    Null,
    /// Custom logger - all messages sent here will be sent on to the logger implementation
    /// you provide.
    Custom(Box<api::Logger>),
}

/// This config struct contains various output options to send messages to.
///
/// You can use this in conjunction with DispatchConfig for message formating and filtering, or
/// alone if you don't need to filter or format messages.
pub struct OutputConfig<'a>(OutputConfigOptions<'a>);

impl <'a> OutputConfig<'a> {
    /// Returns a child logger that sends messages to another DispatchConfig.
    pub fn child(config: DispatchConfig<'a>) -> OutputConfig<'a> {
        return OutputConfig(OutputConfigOptions::Child(config));
    }

    /// Returns a file logger. All messages sent to this will be outputted to the specified path.
    /// Note that the file will be opened with write(true), append(true) and create(true). If you
    /// need to open with other options, use `OutputConfig::file_with_options()`.
    ///
    /// Log files created using this function will use `\n` as the line separator. To specify a
    /// different separator, use `file_with_line_sep`. `file(p)` behaves exactly the same as
    /// `file_with_line_sep(p, "\n")`
    pub fn file<P: ?Sized + AsRef<path::Path>>(path: &'a P) -> OutputConfig<'a> {
        return OutputConfig(OutputConfigOptions::File { path: path.as_ref(), line_sep: "\n" });
    }

    /// Returns a file logger. All messages sent to this will be outputted to the specified path.
    /// Note that the file will be opened with write(true), append(true) and create(true). If you
    /// need to open with other options, use `OutputConfig::file_with_options()`.
    ///
    /// Log files created using this function will use the specified separator as the newline
    /// separator character.
    pub fn file_with_line_sep<P: ?Sized + AsRef<path::Path>>(path: &'a P, line_sep: &'a str)
            -> OutputConfig<'a> {
        return OutputConfig(OutputConfigOptions::File { path: path.as_ref(), line_sep: line_sep });
    }

    /// Returns a file logger with OpenOptions. All messages will be sent to the specified file.
    /// The file will be opened using the specified OpenOptions.
    ///
    /// Log files created using this function will use `\n` as the line separator. To specify a
    /// different separator, use `file_with_options_and_line_sep`. `file_with_options(p, o)`
    /// behaves exactly the same as `file_with_options_and_line_sep(p, o, "\n")`
    pub fn file_with_options<P: ?Sized>(path: &'a P, options: &'a fs::OpenOptions)
            -> OutputConfig<'a> where P: AsRef<path::Path> {
        return OutputConfig(OutputConfigOptions::FileOptions {
            path: path.as_ref(),
            options: options,
            line_sep: "\n",
        });
    }

    /// Returns a file logger with OpenOptions. All messages will be sent to the specified file.
    /// The file will be opened using the specified OpenOptions.
    ///
    /// Log files created using this function will use the specified separator as the newline
    /// separator character.
    pub fn file_with_options_and_line_sep<P: ?Sized>(path: &'a P, options: &'a fs::OpenOptions,
            line_sep: &'a str) -> OutputConfig<'a> where P: AsRef<path::Path> {
        return OutputConfig(OutputConfigOptions::FileOptions {
            path: path.as_ref(),
            options: options,
            line_sep: line_sep,
        });
    }

    /// Returns an stdout logger. All messages sent to this will be printed to stdout.
    pub fn stdout() -> OutputConfig<'static> {
        return OutputConfig(OutputConfigOptions::Stdout);
    }

    /// Returns an stderr logger. All messages sent to this will be printed to stderr.
    pub fn stderr() -> OutputConfig<'static> {
        return OutputConfig(OutputConfigOptions::Stderr);
    }

    /// Returns a null logger. All messages sent to this logger will disappear into the void.
    pub fn null() -> OutputConfig<'static> {
        return OutputConfig(OutputConfigOptions::Null);
    }

    /// Custom implementation logger. All messages sent to this logger will be passed on to your
    /// custom logger.
    pub fn custom(log: Box<api::Logger>) -> OutputConfig<'static> {
        return OutputConfig(OutputConfigOptions::Custom(log));
    }

}

impl <'a> IntoLog for OutputConfig<'a> {
    fn into_fern_logger(self) -> io::Result<Box<api::Logger>> {
        return Ok(match self.0 {
            OutputConfigOptions::Child(config) => try!(config.into_fern_logger()),
            OutputConfigOptions::File{path, line_sep} => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file(path, line_sep))),
            OutputConfigOptions::FileOptions{path, options, line_sep} => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file_with_options(
                    path, options, line_sep))),
            OutputConfigOptions::Stdout => Box::new(
                loggers::WriterLogger::<io::Stdout>::with_stdout()),
            OutputConfigOptions::Stderr => Box::new(
                loggers::WriterLogger::<io::Stderr>::with_stderr()),
            OutputConfigOptions::Null => Box::new(loggers::NullLogger),
            OutputConfigOptions::Custom(log) => log,
        });
    }

    fn into_log(self) -> io::Result<Box<log::Log>> {
        return Ok(match self.0 {
            OutputConfigOptions::Child(config) => try!(config.into_log()),
            OutputConfigOptions::File{path, line_sep} => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file(path, line_sep))),
            OutputConfigOptions::FileOptions{path, options, line_sep} => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file_with_options(
                    path, options, line_sep))),
            OutputConfigOptions::Stdout => Box::new(
                loggers::WriterLogger::<io::Stdout>::with_stdout()),
            OutputConfigOptions::Stderr => Box::new(
                loggers::WriterLogger::<io::Stderr>::with_stderr()),
            OutputConfigOptions::Null => Box::new(loggers::NullLogger),
            OutputConfigOptions::Custom(log) => Box::new(log),
        });
    }
}

impl <'a> IntoLog for DispatchConfig<'a> {
    fn into_fern_logger(self) -> io::Result<Box<api::Logger>> {
        let DispatchConfig {format, level, output} = self;
        let log = try!(loggers::DispatchLogger::new(format, output, level));
        return Ok(Box::new(log));
    }

    fn into_log(self) -> io::Result<Box<log::Log>> {
        let DispatchConfig {format, level, output} = self;
        let log = try!(loggers::DispatchLogger::new(format, output, level));
        return Ok(Box::new(log));
    }
}

impl log::Log for Box<api::Logger> {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        true
    }
    fn log(&self, record: &log::LogRecord) {
        loggers::log_with_fern_logger(self, record);
    }
}

/// Trait which represents any logger configuration which can be built into a `fern::Logger` or
/// `log::Log`.
pub trait IntoLog {
    /// Builds this configuration into a `fern::Logger` that you can send messages to. This method
    /// can be used to generate a logger you can call manually, and catch any errors from. You
    /// probably want to use `fern::init_global_logger()` instead of calling this directly.
    fn into_fern_logger(self) -> io::Result<Box<api::Logger>>;

    /// Builds this configuration into a `log::Log` that you can send messages to. This will open
    /// any files, get handles to stdout/stderr, etc. depending on which type of logger this is.
    fn into_log(self) -> io::Result<Box<log::Log>>;
}

/// Initializes the global logger of the log crate with the specified log configuration. This will
/// return an `InitError(log::SetLoggerError)` if the global logger has already been initialized.
pub fn init_global_logger<L: IntoLog>(config: L, global_log_level: log::LogLevelFilter)
        -> Result<(), InitError> {
    let log = try!(config.into_log());
    try!(log::set_logger(|max_log_level| {
        max_log_level.set(global_log_level);
        log
    }));
    return Ok(());
}
