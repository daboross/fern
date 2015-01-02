use std::io;
use std::sync;
use std::error;

#[deriving(Show)]
pub enum Error {
    Io(io::IoError),
    Poison(String),
}

impl error::FromError<io::IoError> for Error {
    fn from_error(error: io::IoError) -> Error {
        Error::Io(error)
    }
}

impl <T> error::FromError<sync::PoisonError<T>> for Error {
    fn from_error(error: sync::PoisonError<T>) -> Error {
        Error::Poison(format!("{}", error))
    }
}
