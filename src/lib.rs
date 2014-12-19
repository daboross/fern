#![feature(unboxed_closures)]

pub use api::{Logger, Level};
pub use config::{LoggerConfig, OutputConfig};

mod api;
mod config;
mod loggers;
