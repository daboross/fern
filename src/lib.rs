#![feature(unboxed_closures)]

//! Fern is a runtime-configurable rust logging library.

//! Current features are:
//!
//! - Multiple loggers. You can create as many loggers as you need, and configure them separately.
//! - Configurable output format via closures.
//! - Multiple outputs per logger - current options are to a file, or to stdout/stderr (or any combination of those)
//! - Each output can have a Level configured, so you can output all log messages to a log file, and only have warnings and above show up in the console!
//! - You can also define your own custom logging endpoints - have messages end up where you need them!
//! - Thread-local logger storage. This allows for convenient <level>!() and log!() macros via the fern_macros package.
//!
//! fern is still in development, and most features are experimental. The library is subject to change in non-backwards-compatible ways.
//!
//! This library can only be used while complying to the license terms in the `LICENSE` file.
//!
//! Examples
//! ========
//!
//! Usually, the first thing you want to do is create a logger. In fern, you can do this by first
//! creating a LoggerConfig struct, and then calling `.into_logger()` to turn it into a logger.
//!
//! Here's a logger that simply logs all messages to stdout, and an output.log file, formatting
//! each message with the current date, time, and logging level.
//!
//! ```
//! let logger_config = fern::LoggerConfig {
//!     format: box |msg: &str, level: &fern::Level| {
//!         return format!("[{}][{}] {}", chrono::Local::now().format("%Y-%m-%d][%H:%M:%S"), level, msg);
//!     },
//!     output: vec![fern::OutputConfig::Stdout, fern::OutputConfig::File(Path::new("output.log"))],
//!     level: fern::Level::Debug,
//! };
//! ```
//!
//! In the above example, here's what each part does.
//!
//! `format:` is a closure which all messages will be run through. In this example, use a
//! format!() macro to format the messages using the current time, date, and logging level.
//!
//! Side note: `chrono::Local::now().format("%Y-%m-%d][%H:%M:%S")` is a usage of the chrono time
//! library, and just returns the date in the format described. An output of this might look like
//! `2014-12-01][12:55:04`.
//!
//! With this formatting, the final output of the logger will look something like:
//!
//! ```
//! [2014-12-01][12:55:04][INFO] A message logged at the Level::Info logging level.
//! ```
//!
//! `output:` is a Vec<> of other configurations to send the messages to. In this example, we send
//! them to `OutputConfig::Stdout` and `OutputConfig::File(Path::new(config.log_file.as_slice())`.
//! This will send messages to the console, and to a file called "output.log".
//!
//! `level:` is a `fern::Level` which describes the minimum level that should be allowed to pass
//! through this logger. Setting this to `Level::Debug` allows all messages to be logged, as Debug
//! is the lowest logging level.
//!
//! After creating your logging config, you can turn it into a Logger using `into_logger()`:
//!
//! ```
//! let logger = match logger_config.into_logger() {
//!     Some(v) => v,
//!     Err(e) => panic!("Failed to create logger! Error: {}", e),
//! };
//! ```
//!
//! Eventually, this section will contain a few more examples as well. For now though, the above,
//! along with the LoggerConfig and OutputConfig docs, should be enough to get you started on
//! configuring your logger.
//!
//! #### Logging
//!
//! The next part of logging that you'll probably want to do is actually output some messages.
//!
//! The recommended/easiest way to do this is using the `local` module, along with the
//! `fern_macros` crate.
//!
//! To initialize your logger into thread-local storage for use, you can use
//! `fern::local::set_thread_logger()`
//!
//! ```
//! use std::sync;
//!
//! fern::local::set_thread_logger(sync::Arc::new(logger));
//! ```
//!
//! You can then log anywhere in this thread using the following macros from fern_macros:
//!
//! ```
//! #[feature(phase)]
//!
//! // ...
//!
//! #[phase(plugin)]
//! extern crate fern_macros;
//!
//! // ...
//!
//! info!("A message logged at info level");
//!
//! let var = true;
//! debug!("Debugging message containing a variable value: {}", var);
//!
//! warning!("Something bad happened!");
//!
//! severe!("Something really bad happened!");
//! ```
//!
//! If your application is multi-threaded, you can spread the logger across threads as follows:
//!
//! ```
//! use std::sync;
//!
//! let logger_arc = sync::Arc::new(logger); // Create a reference counted logger
//!
//! fern::local::set_thread_logger(logger_arc.clone()); // Initialize the logger for this thread.
//!
//! spawn(move || {
//!     fern::local::set_thread_logger(logger_arc.clone()); // Initialize the logger for this worker thread.
//!
//!     // Do other calculations, and log more here as well.
//! });
//! ```
//!
//! You can also pass the reference counted logger between functions - if you don't want to
//! type out `sync::Arc<Box<fern::Logger + Sync + Send>>` for all the function declarations, you
//! can use `fern::ArcLogger` instead.

pub use api::{Logger, BoxedLogger, ArcLogger, Level};
pub use config::{LoggerConfig, OutputConfig};
pub use loggers::NullLogger;

#[unstable]
pub mod local;
mod api;
mod config;
mod loggers;
