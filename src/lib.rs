#![deny(missing_docs)]
#![doc(html_root_url = "https://dabo.guru/rust/fern/")]
//! Fern: efficient builder based runtime configurable logging.
//!
//! Current features:
//! - Chain loggers indefinitely - each 'Dispatch' can have any number of outputs
//! - Log formatting via formatting closures
//! - Output to stdout, stderr, log files or a custom destination
//! - Configure default minimum log level and log level per-crate for each log destination or for all of them.
//! - All configuration is based off of 'chaining' loggers, so that you can configure filters or levels at each step of
//! the way.
//! - Acts as a backend to the `log` crate - use `trace!()` through `error!()` to log to the global
//!   fern logger.
//!   - Note that fern can also have loggers separate from the global system. You can always set
//!     your main logger as the global logger, then use other fern loggers manually.
//!
//! Although mostly stabilized, fern is still in development. The library is subject to
//! change in non-backwards-compatible ways before the API is completely stabilized.
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
//! log = "0.3"
//! fern = "0.4"
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
//! Usage example
//! ========
//!
//! In fern 0.4, creating, configuring, and establishing a logger as the global logger are all merged
//! into builder methods on the `Dispatch` struct.
//!
//! Here's an example logger which formats messages, limits to Debug level, and puts everything into both stdout and
//! an output.log file.
//!
//! ```no_run
//! # // no_run because this creates an output.log file.
//! extern crate fern;
//! extern crate log;
//! extern crate time;
//!
//! fern::Dispatch::new()
//!     .format(|output, message, record| {
//!         // bring in Write to allow using `write!`
//!         use ::std::fmt::Write;
//!
//!         write!(output, "[{}][{}][{}] {}",
//!                time::now().strftime("%Y-%m-%d][%H:%M:%S").expect("code contained invalid time format"),
//!                record.target(),
//!                record.level(),
//!                message)
//!     })
//!     .level(log::LogLevelFilter::Debug)
//!     .chain(std::io::stdout())
//!     .chain(fern::log_file("output.log").expect("failed to open log file"))
//!     .set_global()
//!     .expect("global logger already initialized");
//! ```
//!
//! Let's unwrap the above example:
//!
//! `fern::Dispatch::new()` creates our logger configuration, with no options and no outputs.
//!
//! `.format(...` adds a formatter to our logger. All messages sent through this logger will be formatted
//! with this format macro. For the best performance, the format macro writes directly to the underlying
//! stream, avoiding any intermediate allocation.
//!
//! If expanded, the type of the format closure would be
//! `|output: &mut std::fmt::Write, message: &std::fmt::Arguments, record: &log::LogRecord| -> std::fmt::Result`
//! - this is left out for simplicity, rust can infer from where you put the closure.
//!
//! `time::now().strftim(...` uses the [`time`] library to make a readable string. The final output
//! of the format will be like:
//!
//! ```text
//! [2015-01-20][12:55:04][crate-name][INFO] Something happened.
//! ```
//!
//! `.level(log::LogLevelFilter::Debug)` sets the minimum level needed to output to Debug. With log's 5-level system,
//! this accepts all messages besides `trace!()`.
//!
//! `.chain(...)` adds a child logger to the dispatch - something that messages which match the level are sent
//! to after being formatted. You can output to any other dispatch, an Stdout or Stderr instance, a File or a
//! `Box<fern::FernLog>` for all other uses.
//!
//! In this example, we chain to stdout and to a log file "output.log".
//!
//! `fern::log_file()` is simply a convenience method to open a writable file without truncating the contents.
//! It is equivalent to `OpenOptions::new().write(true).append(true).create(true).open(x)`
//!
//!
//! Once you've added all the configuration options, `set_global()` consumes the `Dispatch` configuration, constructs
//! a logger and hands that logger off to the `log` crate. It will only fail if you've already initialized a global
//! logger - with fern, or another `log` backend.
//!
//! More documentation can be found on the methods of `fern::Dispatch` - more full examples may be added in the future,
//!  but this should be enough to get you started!
//!
//! #### Logging
//!
//! Ensure you are using `#[macro_use] extern crate log;`, and you should be set! You can use the 5 macros included in
//! `log` to log to your fern logger:
//!
//! ```rust
//! #[macro_use]
//! extern crate log;
//! extern crate fern;
//!
//! # fn main() {
//! # /*
//! fern::Dispatch::new() /*...*/ .set_global();
//! # */
//! # // ignore the error case, we're ok if this isn't the logger we've initialized.
//! # fern::Dispatch::new().set_global().ok();
//!
//! trace!("Trace message");
//! debug!("Debug message");
//! info!("Info message");
//! warn!("Warning message");
//! error!("Error message");
//! # }
//! ```
//!
//! [time]: https://crates.io/crates/time
//! [time-docs]: https://doc.rust-lang.org/time/time/index.html

extern crate log;

pub use log::LogLevelFilter;

use std::convert::AsRef;
use std::path::Path;
use std::fs::{File, OpenOptions};
use std::{io, fmt};

pub use builders::{Dispatch, Output};

mod builders;
mod log_impl;
mod errors;

/// A type alias for a log formatter.
pub type Formatter = Fn(&mut fmt::Write, &fmt::Arguments, &log::LogRecord) -> fmt::Result + Sync + Send;

/// A type alias for a log filter. Returning true means the record should succeed - false means it should fail.
pub type Filter = Fn(&log::LogMetadata) -> bool + Send + Sync;

/// Fern logging trait. This is necessary in order to allow for custom loggers taking in arguments that have already had
/// a custom format applied to them.
///
/// The original `log::Log` trait's `log` method only accepts messages that were created using the log macros - this
/// trait also accepts records which have had additional formatting applied to them.
pub trait FernLog: Sync + Send {
    /// Logs a log record, but with the given fmt::Arguments instead of the one contained in the LogRecord.
    ///
    /// This has access to the original record, but _should ignore_ the original `record.args()` and instead
    /// use the passed in payload.
    fn log_args(&self, payload: &fmt::Arguments, record: &log::LogRecord);
}

/// Utility for opening a log file with write, create and append options.
///
/// Exactly equivalent to
/// `std::fs::OpenOptions::new().write(true).append(true).truncate(false).create(true).open(path)`.
#[inline]
pub fn log_file<P: AsRef<Path>>(path: P) -> Result<File, io::Error> {
    OpenOptions::new().write(true).append(true).truncate(false).create(true).open(path)
}
