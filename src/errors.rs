use std::io;
use std::sync;
use std::error;
use std::convert;
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

impl convert::From<io::Error> for LogError {
    fn from(error: io::Error) -> LogError {
        LogError::Io(error)
    }
}

impl <T> convert::From<sync::PoisonError<T>> for LogError {
    fn from(error: sync::PoisonError<T>) -> LogError {
        LogError::Poison(format!("{}", error))
    }
}

impl error::Error for LogError {
    fn description(&self) -> &str {
        match self {
            &LogError::Io(..) => "IO error while logging",
            &LogError::Poison(..) => "lock within logger poisoned",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            &LogError::Io(ref e) => Some(e),
            &LogError::Poison(..) => None,
        }
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

/// Error that may occur within init_global_logger()
#[derive(Debug)]
pub enum InitError {
    /// IO Error - this will only occur within fern logger implementations when opening files
    Io(io::Error),
    /// SetLoggerError - this occurs if the log crate has already been initialized when
    /// init_global_logger() is called.
    SetLoggerError(log::SetLoggerError),
}

impl convert::From<io::Error> for InitError {
    fn from(error: io::Error) -> InitError {
        InitError::Io(error)
    }
}

impl convert::From<log::SetLoggerError> for InitError {
    fn from(error: log::SetLoggerError) -> InitError {
        InitError::SetLoggerError(error)
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &InitError::Io(ref e) => write!(f, "IO Error: {}", e),
            &InitError::SetLoggerError(ref e) => write!(f, "SetLoggerError: {}", e),
        }
    }
}

impl error::Error for InitError {
    fn description(&self) -> &str {
        match self {
            &InitError::Io(..) => "IO error while initializing",
            &InitError::SetLoggerError(..) => "global logger already initialized",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            &InitError::Io(ref e) => Some(e),
            &InitError::SetLoggerError(ref e) => Some(e),
        }
    }
}
