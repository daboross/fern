use std::io;
use std::sync;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum LogError {
    Io(io::Error),
    Poison(String),
}

impl error::FromError<io::Error> for LogError {
    fn from_error(error: io::Error) -> LogError {
        LogError::Io(error)
    }
}

impl <T> error::FromError<sync::PoisonError<T>> for LogError {
    fn from_error(error: sync::PoisonError<T>) -> LogError {
        LogError::Poison(format!("{}", error))
    }
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &LogError::Io(ref e) => write!(f, "IO Error: {}", e),
            &LogError::Poison(ref e) => write!(f, "Poison Error: {}", e),
        }
    }
}
