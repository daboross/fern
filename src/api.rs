use Level;
use std::io;

pub trait Logger {
    fn log(&self, level: Level, message: &str) -> io::IoResult<()>;
}

pub trait IntoLogger {
    fn into_logger(self) -> io::IoResult<Box<Logger + 'static>>;
}
