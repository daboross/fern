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
    pub fn new(format: Box<config::Formatter>,
               config_output: Vec<config::OutputConfig>,
               level: log::LogLevelFilter)
               -> io::Result<Self> {

        let output = config_output.into_iter()
            .map(config::OutputConfig::into_fern_logger)
            .collect::<io::Result<Vec<_>>>()?;

        Ok(DispatchLogger {
            output: output,
            level: level,
            format: format,
        })
    }
}

impl api::Logger for DispatchLogger {
    fn log(&self, msg: &str, level: &log::LogLevel, location: &log::LogLocation) -> Result<(), LogError> {
        if *level > self.level {
            return Ok(());
        }

        let new_msg = (self.format)(msg, level, location);
        for logger in &self.output {
            logger.log(&new_msg, level, location)?;
        }

        Ok(())
    }
}

impl log::Log for DispatchLogger {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::LogRecord) {
        // shortstop for checking level here, so we don't have to do any conversions in
        // log_with_fern_logger
        if record.level() <= self.level {
            log_with_fern_logger(self, record);
        }
    }
}

pub struct WriterLogger<T: io::Write + Send> {
    writer: sync::Arc<sync::Mutex<T>>,
    line_sep: String,
}

impl<T: io::Write + Send> WriterLogger<T> {
    pub fn new(writer: T, line_sep: &str) -> WriterLogger<T> {
        WriterLogger {
            writer: sync::Arc::new(sync::Mutex::new(writer)),
            line_sep: line_sep.to_string(),
        }
    }

    pub fn with_stdout() -> WriterLogger<io::Stdout> {
        WriterLogger::new(io::stdout(), "\n")
    }

    pub fn with_stderr() -> WriterLogger<io::Stderr> {
        WriterLogger::new(io::stderr(), "\n")
    }

    pub fn with_file(path: &path::Path, line_sep: &str) -> io::Result<WriterLogger<fs::File>> {
        Ok(WriterLogger::new(fs::OpenOptions::new().write(true).append(true).create(true).open(path)?,
                             line_sep))
    }

    pub fn with_file_with_options(path: &path::Path,
                                  options: &fs::OpenOptions,
                                  line_sep: &str)
                                  -> io::Result<WriterLogger<fs::File>> {
        Ok(WriterLogger::new(options.open(path)?, line_sep))
    }
}

impl<T: io::Write + Send> api::Logger for WriterLogger<T> {
    fn log(&self, msg: &str, _level: &log::LogLevel, _location: &log::LogLocation) -> Result<(), LogError> {
        write!(self.writer.lock()?, "{}{}", msg, self.line_sep)?;

        Ok(())
    }
}

impl<T: io::Write + Send> log::Log for WriterLogger<T> {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &log::LogRecord) {
        log_with_fern_logger(self, record)
    }
}

/// A logger implementation which does nothing with logged messages.
#[derive(Clone, Copy)]
pub struct NullLogger;

impl api::Logger for NullLogger {
    fn log(&self, _msg: &str, _level: &log::LogLevel, _location: &log::LogLocation) -> Result<(), LogError> {
        Ok(())
    }
}

impl log::Log for NullLogger {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        false
    }

    fn log(&self, record: &log::LogRecord) {
        log_with_fern_logger(self, record)
    }
}

/// Implementation of log::Log::log for any type which implements fern::Logger.
pub fn log_with_fern_logger<T>(logger: &T, record: &log::LogRecord)
    where T: api::Logger
{
    let args_formatted = format!("{}", record.args());

    if let Err(e) = api::Logger::log(logger, &args_formatted, &record.level(), record.location()) {
        let backup_result = write!(&mut io::stderr(),
                                   "Error logging {{level: {}, location: {:?}, arguments: {}}}: {:?}",
                                   record.level(), record.location(), args_formatted, e);

        if let Err(e2) = backup_result {
            panic!("Backup logging failed after regular logging failed.
                    \nLog record: {{level: {}, location: {:?}, arguments: {}}}
                    \nLogging error: {:?}
                    \nBackup logging error: {}",
                   record.level(), record.location(), args_formatted, e, e2);
        }
    }
}
