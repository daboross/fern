#![deny(missing_docs)]
#![doc(html_root_url = "http://dabo.guru/rust/fern/")]
//! Fern is a runtime-configurable rust logging library.
//!
//! Current features:
//!
//! - Multiple loggers. You can create as many loggers as you need, and configure them separately.
//! - Configurable output format via closures.
//! - Multiple outputs per logger - output to any combination of:
//!   - log files
//!   - stdout or stderr
//!   - your own custom implementation
//! - Each output can have a Level configured, so you can output all log messages to a log file,
//!   and only have warnings and above show up in the console.
//! - You can also define your own custom logging endpoints - have messages end up where you need
//!   them.
//! - Acts as a backend to the `log` crate - use `trace!()` through `error!()` to log to the global
//!   fern logger.
//!   - Note that fern can also have loggers separate from the global system. You can always set
//!     your main logger as the global logger, then use other fern loggers manually.
//!
//! Although mostly stabilized, fern is still in development. The library is subject to
//! change in non-backwards-compatible ways before the API is completely stabilized.
//!
//! This library can only be used while complying to the license terms in the `LICENSE` file.
//!
//! Upgrade note for fern `0.2.*`
//! ====
//!
//! As of fern 0.2.0, fern depends on the `log` crate to provide the frontend logging macros and
//! logging levels - you will need to depend on and use both the `fern` and `log` crates.
//!
//!
//! Adding fern as a dependency
//! ===========================
//!
//! In order to use fern, the first thing you'll need to add both the `fern` crate and the `log`
//! crate to the dependencies in your project's `Cargo.toml` file:
//!
//! ```toml
//! # ...
//!
//! [dependencies]
//! # ...
//! log = "0.2.*"
//! fern = "0.2.*"
//!
//! # ...
//! ```
//!
//! After this, you'll need to declare the `log` and `fern` at the top of your main.rs or lib.rs
//! file:
//!
//! ```
//! #[macro_use]
//! extern crate log;
//! extern crate fern;
//! # fn main() {}
//! ```
//!
//! Examples
//! ========
//!
//!
//! Usually, the first thing you want to do is create a logger. In fern, you can do this by first
//! creating a LoggerConfig struct, and then calling `.into_logger()` to turn it into a logger.
//!
//! Here's a logger that simply logs all messages to stdout, and an output.log file, formatting
//! each message with the current date, time, and logging level.
//!
//! ```ignore
//! # // as a workaround for https://github.com/rust-lang/cargo/issues/1474, this test is copied
//! # // into `tests/doc_test_copy.rs` and tested there. Any changes to this doc code block should
//! # // be duplicated there.
//! extern crate fern;
//! extern crate log;
//! extern crate time;
//!
//! let logger_config = fern::DispatchConfig {
//!     format: Box::new(|msg: &str, level: &log::LogLevel, _location: &log::LogLocation| {
//!         // This is a fairly simple format, though it's possible to do more complicated ones.
//!         // This closure can contain any code, as long as it produces a String message.
//!         format!("[{}][{}] {}", time::now().strftime("%Y-%m-%d][%H:%M:%S").unwrap(), level, msg)
//!     }),
//!     output: vec![fern::OutputConfig::stdout(), fern::OutputConfig::file("output.log")],
//!     level: log::LogLevelFilter::Trace,
//! };
//! ```
//!
//! In the above example, here's what each part does.
//!
//! `format:` is a closure which all messages will be run through. In this example, use a
//! format!() macro to format the messages using the logging level.
//!
//! Side note: `time::now().strftime("%Y-%m-%d][%H:%M:%S")` is a usage of the `time` library to
//! format the current date/time into a readable string. This particular format will produce a
//! string akin to `2015-01-20][12:55:04`
//!
//! With this formatting, the final output of the logger will look something like:
//!
//! ```text
//! [2015-01-20][12:55:04][INFO] A message logged at the Info logging level.
//! ```
//!
//! `output:` is a Vec<> of other configurations to send the messages to. In this example, we send
//! messages to stdout (the console), and the file "output.log".
//!
//! `level:` is a `log::LogLevelFilter` which describes the minimum level that should be allowed
//! to pass through this logger. Setting this to `LogLevelFilter::Trace` allows all messages to be
//! logged, as `Trace` is the lowest logging level.
//!
//! After creating your logging config, you can pass it to `init_global_logger` to set it as the
//! logger used with logging macros in the `log` crate. This function can only be called once, as
//! the log crate may only be set up once.
//!
//! Note that this function also accepts a `LogLevelFilter`. This is so that you can set a global
//! minimum log level separate from the logger configuration. If you don't have any reason for
//! anything else, just set this to `log::LogLevelFilter::Trace`.
//!
//! ```rust
//! # extern crate fern;
//! # extern crate log;
//! # let logger_config = fern::DispatchConfig {
//! #     format: Box::new(|msg: &str, _level: &log::LogLevel, _location: &log::LogLocation| {
//! #         format!("{}", msg)
//! #     }),
//! #     output: vec![],
//! #     level: log::LogLevelFilter::Trace,
//! # };
//! if let Err(e) = fern::init_global_logger(logger_config, log::LogLevelFilter::Trace) {
//!     panic!("Failed to initialize global logger: {}", e);
//! }
//! ```
//!
//! This uses the `if let Err(e) =` syntax to catch any errors that happen when initializing
//! the logger.
//!
//!
//! Eventually, this section will contain a few more examples as well. For now though, the above
//! tutorial paired with the fern docs should be enough to get you started on configuring a logger
//! for your project.
//!
//! #### Logging
//!
//! With the `log` crate, outputting messages is fairly simple. As long as you are using the log
//! crate with `#[macro_use]`, and you have initialized the global logger as shown above, you can
//! use the log macros to log messages.
//!
//! ```rust
//! // You need to have #[macro_use] to use log macros
//! #[macro_use]
//! extern crate log;
//! extern crate fern;
//!
//! # fn main() {
//! # /*
//! fern::init_global_logger(...);
//! # */
//! # fern::init_global_logger(fern::OutputConfig::null(), log::LogLevelFilter::Trace);
//!
//! trace!("Trace message");
//! debug!("Debug message");
//! info!("Info message");
//! warn!("Warning message");
//! error!("Error message");
//! # }
//! ```

extern crate log;

pub use errors::{LogError, InitError};
pub use api::Logger;
pub use config::{DispatchConfig, OutputConfig, IntoLog, init_global_logger};
pub use loggers::NullLogger;

mod api;
mod config;
mod loggers;
mod errors;
