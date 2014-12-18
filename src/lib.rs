#![feature(unboxed_closures)]

use std::fmt;

pub use api::IntoLogger;
pub use api::Logger;
pub use config::Logger as LoggerConfig;
pub use config::Output as LoggerOutput;

mod config;
mod api;
mod loggers;

pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl Copy for Level {}

impl Level {
    pub fn as_int(&self) -> u8 {
        match self {
            &Level::Debug => 0u8,
            &Level::Info => 1u8,
            &Level::Warning => 2u8,
            &Level::Error => 3u8,
            &Level::Critical => 4u8,
        }
    }
}

impl fmt::Show for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}", match self {
            &Level::Debug => "DEBUG",
            &Level::Info => "INFO",
            &Level::Warning => "WARNING",
            &Level::Error => "ERROR",
            &Level::Critical => "CRITICAL",
        });
    }
}
