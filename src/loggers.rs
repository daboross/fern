use std::io;
use std::sync;
use std::fs;
use std::path;

use config::IntoLog;
use errors::LogError;
use api;
use config;

pub struct DispatchLogger {
    pub output: Vec<Box<api::Logger>>,
    pub level: api::Level,
    pub format: Box<config::Formatter>,
}

impl DispatchLogger {
    pub fn new(format: Box<config::Formatter>, config_output: Vec<config::OutputConfig>,
            level: api::Level) -> io::Result<DispatchLogger> {

        let output = try!(config_output.into_iter().fold(Ok(Vec::new()),
                     |processed: io::Result<Vec<Box<api::Logger>>>, next: config::OutputConfig| {
            // If an error has already been found, don't try to process any future outputs, just
            // continue passing along the error.
            let mut processed_so_far = try!(processed);
            return match next.into_logger() {
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
    fn log(&self, msg: &str, level: &api::Level) -> Result<(), LogError> {
        if *level < self.level {
            return Ok(());
        }
        let new_msg = (self.format)(msg, level);
        for logger in &self.output {
            try!(logger.log(&new_msg, level));
        }
        return Ok(());
    }
}

pub struct WriterLogger<T: io::Write> {
    writer: sync::Arc<sync::Mutex<T>>,
}

impl <T: io::Write + Send> WriterLogger<T> {
    pub fn new(writer: T) -> WriterLogger<T> {
        return WriterLogger {
            writer: sync::Arc::new(sync::Mutex::new(writer)),
        };
    }

    pub fn with_stdout() -> WriterLogger<io::Stdout> {
        return WriterLogger::new(io::stdout());
    }

    pub fn with_stderr() -> WriterLogger<io::Stderr> {
        return WriterLogger::new(io::stderr());
    }

    pub fn with_file<P: path::AsPath + ?Sized>(path: &P) -> io::Result<WriterLogger<fs::File>> {
        return Ok(WriterLogger::new(try!(fs::OpenOptions::new().write(true).append(true)
                                            .create(true).open(path))));
    }
}

impl <T: io::Write + Send> api::Logger for WriterLogger<T> {
    fn log(&self, msg: &str, _level: &api::Level) -> Result<(), LogError> {
        try!(write!(try!(self.writer.lock()), "{}\n", msg));
        return Ok(());
    }
}

/// A logger implementation which does nothing with logged messages.
#[unstable]
#[derive(Copy)]
pub struct NullLogger;

impl api::Logger for NullLogger {
    fn log(&self, _msg: &str, _level: &api::Level) -> Result<(), LogError> {
        return Ok(());
    }
}
