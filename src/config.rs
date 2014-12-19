use std::io;
use std::io::stdio;

use api;
use loggers;

pub struct LoggerConfig {
    pub format: Box<Fn(&str, &api::Level) -> String + Sync + Send>,
    pub output: Vec<OutputConfig>,
    pub level: api::Level,
}

pub enum OutputConfig {
    Parent(LoggerConfig),
    File(Path),
    Stdout,
    Stderr,
    Custom(Box<api::Logger + Sync + Send>),
}

impl OutputConfig {
    pub fn into_logger(self) -> io::IoResult<Box<api::Logger + Sync + Send>> {
        return Ok(match self {
            OutputConfig::Parent(config) => try!(config.into_logger()),
            OutputConfig::File(ref path) => box try!(loggers::WriterLogger::<io::File>::with_file(path)) as Box<api::Logger + Sync + Send>,
            OutputConfig::Stdout => box loggers::WriterLogger::<stdio::StdWriter>::with_stdout() as Box<api::Logger + Sync + Send>,
            OutputConfig::Stderr => box loggers::WriterLogger::<stdio::StdWriter>::with_stderr() as Box<api::Logger + Sync + Send>,
            OutputConfig::Custom(log) => log,
        });
    }
}

impl LoggerConfig {
    pub fn into_logger(self) -> io::IoResult<Box<api::Logger + Sync + Send>> {
        let LoggerConfig {format, level, output} = self;
        let log = try!(loggers::ConfigurationLogger::new(format, output, level));
        return Ok(box log as Box<api::Logger + Sync + Send>);
    }
}
