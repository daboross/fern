#![unstable]
use std::io;
use std::sync;
use std::error;
use std::fmt;

use log;

/// Error that may occur within logging
#[derive(Debug)]
pub enum LogError {
    /// IO Error
    Io(io::Error),
    /// Poison error - this will only occur within fern logger implementations if write!() panics.
    Poison(String),
}

impl error::FromError<io::Error> for LogError {
    fn from_error(error: io::Error) -> LogError {
        LogError::Io(error)
    }
}
#[unstable]
impl <T> error::FromError<sync::PoisonError<T>> for LogError {
    fn from_error(error: sync::PoisonError<T>) -> LogError {
        LogError::Poison(format!("{}", error))
    }
}

#[stable]
impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &LogError::Io(ref e) => write!(f, "IO Error: {}", e),
            &LogError::Poison(ref e) => write!(f, "Poison Error: {}", e),
        }
    }
}

/// Error that may occur within init_global_logger()
#[derive(Debug)]
pub enum InitError {
    /// IO Error - this will only occur within fern logger implementations when opening files
    Io(io::Error),
    /// SetLoggerError - this occurs if the log crate has already been initialized when
    /// init_global_logger() is called.
    SetLoggerError(log::SetLoggerError),
}

impl error::FromError<io::Error> for InitError {
    fn from_error(error: io::Error) -> InitError {
        InitError::Io(error)
    }
}

impl error::FromError<log::SetLoggerError> for InitError {
    fn from_error(error: log::SetLoggerError) -> InitError {
        InitError::SetLoggerError(error)
    }
}

#[stable]
impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &InitError::Io(ref e) => write!(f, "IO Error: {}", e),
            &InitError::SetLoggerError(ref e) => write!(f, "SetLoggerError: {}", e),
        }
    }
}
