use std::io;
use std::sync;
use std::error;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Poison(String),
}

impl error::FromError<io::Error> for Error {
    fn from_error(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl <T> error::FromError<sync::PoisonError<T>> for Error {
    fn from_error(error: sync::PoisonError<T>) -> Error {
        Error::Poison(format!("{}", error))
    }
}
