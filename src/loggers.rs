use std::sync::{Arc, Mutex};

use config;
use api::Logger;
use api::IntoLogger;
use std::io;
use std::io::stdio;
use Level;


struct ConfigurationLogger {
    output: Vec<Box<Logger + 'static>>,
    level: Level,
    format: Box<Fn(&str) -> String + Send>,
}

impl Logger for ConfigurationLogger {
    fn log(&self, level: Level, msg: &str) -> io::IoResult<()> {
        if level.as_int() < self.level.as_int() {
            return Ok(());
        }
        let new_msg = self.format.call((msg,));
        for logger in self.output.iter() {
            try!(logger.log(level, new_msg.as_slice()));
        }
        return Ok(());
    }
}

impl IntoLogger for config::Output {
    fn into_logger(self) -> io::IoResult<Box<Logger + 'static>> {
        return Ok(match self {
            config::Output::Parent(config) => try!(config.into_logger()),
            config::Output::File(ref path) => box try!(WriterLogger::<io::File>::with_file(path)) as Box<Logger>,
            config::Output::Stdout => box WriterLogger::<io::LineBufferedWriter<stdio::StdWriter>>::with_stdout() as Box<Logger>,
            config::Output::Stderr => box WriterLogger::<io::LineBufferedWriter<stdio::StdWriter>>::with_stderr() as Box<Logger>,
        });
    }
}

impl IntoLogger for config::Logger {
    fn into_logger(self) -> io::IoResult<Box<Logger + 'static>> {
        let config::Logger {
            format,
            level,
            output: config_output,
        } = self;

        let output: Vec<Box<Logger>> = try!(config_output.into_iter().fold(Ok(Vec::new()),
                                            |processed: io::IoResult<Vec<Box<Logger + 'static>>>, next: config::Output| {
            // If an error has already been found, don't try to process any future outputs, just continue passing along the error.
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
        return Ok(box ConfigurationLogger {
            output: output,
            level: level,
            format: format,
        } as Box<Logger>);
    }
}

struct WriterLogger<T: io::Writer + Send> {
    writer: Arc<Mutex<T>>,
}

impl <T: io::Writer + Send> WriterLogger<T> {
    fn new(writer: T) -> WriterLogger<T> {
        return WriterLogger {
            writer: Arc::new(Mutex::new(writer)),
        };
    }

    fn with_stdout() -> WriterLogger<io::LineBufferedWriter<io::stdio::StdWriter>> {
        return WriterLogger::new(io::stdout());
    }

    fn with_stderr() -> WriterLogger<io::LineBufferedWriter<io::stdio::StdWriter>> {
        return WriterLogger::new(io::stderr());
    }

    fn with_file(path: &Path) -> io::IoResult<WriterLogger<io::File>> {
        return Ok(WriterLogger::new(try!(io::File::create(path))));
    }
}

impl <T: io::Writer + Send> Logger for WriterLogger<T> {
    fn log(&self, _level: Level, message: &str) -> io::IoResult<()> {
        return self.writer.lock().write_line(message);
    }
}
