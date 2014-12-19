#![feature(unboxed_closures)]

pub use api::{
    Logger,
    IntoLogger,
    Level,
};
pub use config::Logger as LoggerConfig;
pub use config::Output as LoggerOutput;

mod api;
mod config;
mod loggers;
