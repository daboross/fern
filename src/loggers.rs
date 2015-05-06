use std::io::Write;
use std::io;
use std::sync;
use std::fs;
use std::path;

use log;

use config::IntoLog;
use errors::LogError;
use api;
use config;

pub struct DispatchLogger {
    pub output: Vec<Box<api::Logger>>,
    pub level: log::LogLevelFilter,
    pub format: Box<config::Formatter>,
}

impl DispatchLogger {
    pub fn new(format: Box<config::Formatter>, config_output: Vec<config::OutputConfig>,
            level: log::LogLevelFilter) -> io::Result<DispatchLogger> {

        let output = try!(config_output.into_iter().fold(Ok(Vec::new()),
                     |processed: io::Result<Vec<Box<api::Logger>>>, next: config::OutputConfig| {
            // If an error has already been found, don't try to process any future outputs, just
            // continue passing along the error.
            let mut processed_so_far = try!(processed);
            return match next.into_fern_logger() {
                Err(e) => Err(e), // If this one errors, return the error instead of the Vec so far
                Ok(processed_value) => {
                    // If it's ok, add the processed logger to the vec, and pass the vec along
                    processed_so_far.push(processed_value);
                    Ok(processed_so_far)
                }
            };
        }));

        return Ok(DispatchLogger {
            output: output,
            level: level,
            format: format,
        });
    }
}

impl api::Logger for DispatchLogger {
    fn log(&self, msg: &str, level: &log::LogLevel, location: &log::LogLocation)
            -> Result<(), LogError> {
        if *level > self.level {
            return Ok(());
        }
        let new_msg = (self.format)(msg, level, location);
        for logger in &self.output {
            try!(logger.log(&new_msg, level, location));
        }
        return Ok(());
    }
}

impl log::Log for DispatchLogger {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::LogRecord) {
        // shortstop for checking level here, so we don't have to do any conversions in
        // log_with_fern_logger
        if record.level() > self.level {
            return;
        }
        log_with_fern_logger(self, record);
    }
}

pub struct WriterLogger<T: io::Write + Send> {
    writer: sync::Arc<sync::Mutex<T>>,
    line_sep: String,
}

impl <T: io::Write + Send> WriterLogger<T> {
    pub fn new(writer: T, line_sep: &str) -> WriterLogger<T> {
        return WriterLogger {
            writer: sync::Arc::new(sync::Mutex::new(writer)),
            line_sep: line_sep.to_string(),
        };
    }

    pub fn with_stdout() -> WriterLogger<io::Stdout> {
        return WriterLogger::new(io::stdout(), "\n");
    }

    pub fn with_stderr() -> WriterLogger<io::Stderr> {
        return WriterLogger::new(io::stderr(), "\n");
    }

    pub fn with_file(path: &path::Path, line_sep: &str) -> io::Result<WriterLogger<fs::File>> {
        return Ok(WriterLogger::new(try!(fs::OpenOptions::new().write(true).append(true)
                                            .create(true).open(path)), line_sep));
    }

    pub fn with_file_with_options(path: &path::Path, options: &fs::OpenOptions, line_sep: &str)
            -> io::Result<WriterLogger<fs::File>> {
        return Ok(WriterLogger::new(try!(options.open(path)), line_sep));
    }
}

impl <T: io::Write + Send> api::Logger for WriterLogger<T> {
    fn log(&self, msg: &str, _level: &log::LogLevel, _location: &log::LogLocation)
            -> Result<(), LogError> {
        try!(write!(try!(self.writer.lock()), "{}{}", msg, self.line_sep));
        return Ok(());
    }
}

impl <T: io::Write + Send> log::Log for WriterLogger<T> {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &log::LogRecord) {
        log_with_fern_logger(self, record);
    }
}

/// A logger implementation which does nothing with logged messages.
#[derive(Clone, Copy)]
pub struct NullLogger;

impl api::Logger for NullLogger {
    fn log(&self, _msg: &str, _level: &log::LogLevel, _location: &log::LogLocation)
            -> Result<(), LogError> {
        return Ok(());
    }
}

impl log::Log for NullLogger {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        false
    }

    fn log(&self, record: &log::LogRecord) {
        log_with_fern_logger(self, record);
    }
}

/// Implementation of log::Log::log for any type which implements fern::Logger.
pub fn log_with_fern_logger<T>(logger: &T, record: &log::LogRecord) where T: api::Logger {
    let args_formatted = format!("{}", record.args());
    if let Err(e) = api::Logger::log(logger, &args_formatted, &record.level(), record.location()) {
        let backup_result = write!(&mut io::stderr(),
                "Error logging {{level: {}, location: {:?}, arguments: {}}}: {:?}",
                record.level(), record.location(), args_formatted, e);
        if let Err(e2) = backup_result {
            panic!(format!(
                "Backup logging failed after regular logging failed.\n\
                Log record: {{level: {}, location: {:?}, arguments: {}}}\n\
                Logging error: {:?}\n\
                Backup logging error: {}",
                record.level(), record.location(), args_formatted, e, e2));
        }
    }
}
