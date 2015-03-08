use std::io;
use std::sync;
use std::error;

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
