//! This file manages storing thread-local loggers
//!
//! The current logger can be accessed with the `log()` function to log something to the logger,
//! or by using the macros in the `fern_macros` package.

use std::cell;
use std::sync;

use errors::Error;
use api;
use config;

thread_local!{
    static THREAD_LOGGER: cell::RefCell<api::ArcLogger> = cell::RefCell::new(sync::Arc::new(
        config::OutputConfig::Stdout.into_logger().unwrap()))
}

/// This function sets the current thread's thread-local logger.
/// This takes an ArcLogger instead of just a Logger, so as to allow you to initialize your logger
/// once, and then use it on as many threads as necessary.
///
/// The logger set can be accessed directly using the `log()` function.
#[unstable]
pub fn set_thread_logger(logger: api::ArcLogger) {
    THREAD_LOGGER.with(move |thread_logger| {
        *thread_logger.borrow_mut() = logger;
    });
}

/// Logs something to the thread-local logger set via set_thread_logger. If no logger has been set,
/// `log()` defaults to using a logger outputting to stdout.
///
/// For a more friendly interface which will automatically report errors, and allows inline
/// formatting, try using the `log!()` macro in the `fern_macros` package.
#[unstable]
pub fn log(level: &api::Level, msg: &str) -> Result<(), Error> {
    THREAD_LOGGER.with(|logger| {
        logger.borrow().log(level, msg)
    })
}
