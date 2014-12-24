#![feature(unboxed_closures)]

//! Fern is a runtime-configurable rust logging library.

pub use api::{Logger, BoxedLogger, ArcLogger, Level};
pub use config::{LoggerConfig, OutputConfig};

mod api;
mod config;
mod loggers;
