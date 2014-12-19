use std::io;
use std::fmt;

pub trait Logger {
    fn log(&self, level: &Level, message: &str) -> io::IoResult<()>;
}

#[deriving(Copy)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Severe,
}

impl Level {
    pub fn as_int(&self) -> u8 {
        match self {
            &Level::Debug => 0u8,
            &Level::Info => 1u8,
            &Level::Warning => 2u8,
            &Level::Severe => 3u8,
        }
    }
}

impl fmt::Show for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}", match self {
            &Level::Debug => "DEBUG",
            &Level::Info => "INFO",
            &Level::Warning => "WARNING",
            &Level::Severe => "SEVERE",
        });
    }
}
