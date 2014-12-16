#![feature(macro_rules)]
#![feature(unboxed_closures)]

pub use config::Logger as LoggerConfig;
pub use config::Output as LoggerOutput;
pub use api::Logger;

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
