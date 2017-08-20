#![deny(missing_docs)]
#![doc(html_root_url = "https://dabo.guru/rust/fern/")]
//! Efficient, configurable logging in Rust.
//!
//! With Fern, you can:
//!
//! - Configure logging at runtime; make changes based off of user arguments or configuration
//! - Format log records without allocating intermediate results
//! - Output to stdout, stderr, log files and custom destinations
//! - Apply a blanket level filter and per-crate/per-module overrides
//! - Intuitively apply filters and formats to groups of loggers via builder chaining
//! - Log using the standard `log` crate macros
//!
//! # Depending on `fern`
//!
//! First, depend on the `fern` and `log` crates in your project's `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! log = "0.3"
//! fern = "0.4"
//! ```
//!
//! Then declare both in your program's `main.rs` or `lib.rs`:
//!
//! ```
//! #[macro_use]
//! extern crate log;
//! extern crate fern;
//! # fn main() {}
//! ```
//!
//! # Example setup:
//!
//! In fern 0.4, creating, configuring, and establishing a logger as the global logger are all merged
//! into builder methods on the `Dispatch` struct.
//!
//! Here's an example logger which formats messages, limits to Debug level, and puts everything into both stdout and
//! an output.log file.
//!
//! ```no_run
//! extern crate chrono;
//! # extern crate fern;
//! # #[macro_use]
//! # extern crate log;
//!
//! # fn setup_logger() -> Result<(), fern::InitError> {
//! fern::Dispatch::new()
//!     .format(|out, message, record| {
//!         out.finish(format_args!(
//!             "{}[{}][{}] {}",
//!             chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
//!             record.target(),
//!             record.level(),
//!             message
//!         ))
//!     })
//!     .level(log::LogLevelFilter::Debug)
//!     .chain(std::io::stdout())
//!     .chain(fern::log_file("output.log")?)
//!     .apply()?;
//! # Ok(())
//! # }
//! # fn main() {
//! #     setup_logger().expect("failed to set up logger")
//! # }
//! ```
//!
//! Let's unwrap the above example:
//!
//! ---
//!
//! [`fern::Dispatch::new()`]
//!
//! Create an empty logger config.
//!
//! ---
//!
//! [`.format(|...| ...)`]
//!
//! Add a formatter to the logger, modifying all messages sent through.
//!
//! ___
//!
//! [`chrono::Local::now()`]
//!
//! Get the current time in the local timezone using the [`chrono`] library. See the [time-and-date docs].
//!
//! ___
//!
//! [`.format(`]`"[%Y-%m-%d][%H:%M:%S]`[`")`]
//!
//! Use chrono's lazy format specifier to turn the time into a readable string.
//!
//! ---
//!
//! [`out.finish(format_args!(...))`]
//!
//! Call the `fern::FormattingCallback` to submit the formatted message.
//!
//! Fern uses this callback style to allow usage of [`std::fmt`] formatting without the allocation that would
//! be needed to return the formatted result.
//!
//! [`format_args!()`] has the same arguments as [`println!()`] or any other [`std::fmt`]-based macro.
//!
//! The final output of this formatter will be:
//!
//! ```text
//! [2017-01-20][12:55:04][crate-name][INFO] Something happened.
//! ```
//!
//! ---
//!
//! [`.level(log::LogLevelFilter::Debug)`]
//!
//! Set the minimum level needed to output to `Debug`.
//!
//! This accepts `Debug`, `Info`, `Warn` and `Error` level messages, and denies the lowest level, `Trace`.
//!
//! ---
//!
//! [`.chain(std::io::stdout())`]
//!
//! Add a child to the logger; send all messages to stdout.
//!
//! [`Dispatch::chain`] accepts [`Stdout`], [`Stderr`], [`File`]s and other [`Dispatch`] instances.
//!
//! ---
//!
//! [`.chain(fern::log_file(...)?)`]
//!
//! Add a second child; send all messages to the file "output.log".
//!
//! See [`fern::log_file()`] for more info on file output.
//!
//! ---
//!
//! [`.apply()`][`.apply`]
//!
//! Consume the configuration and instantiate it as the current runtime global logger.
//!
//! This will fail if and only if another fern or [`log`] logger has already been set as the global logger.
//!
//! Since it's really up to the binary-producing crate to set up logging, the [`apply`] result can be reasonably
//! unwrapped or ignored.
//!
//! # Logging
//!
//! Once the logger has been set using [`apply`] it will pick up all [`log`] macro calls from your
//! crate and all your libraries.
//!
//! ```rust
//! #[macro_use]
//! extern crate log;
//! extern crate fern;
//!
//! # fn setup_logger() -> Result<(), fern::InitError> {
//! fern::Dispatch::new()
//!     // ...
//!     .apply()?;
//! # Ok(())
//! # }
//!
//! # fn main() {
//! # setup_logger().ok(); // we're ok with this not succeeding.
//! trace!("Trace message");
//! debug!("Debug message");
//! info!("Info message");
//! warn!("Warning message");
//! error!("Error message");
//! # }
//! ```
//!
//! # More configuration
//!
//! Check out the [`Dispatch` documentation] and the [full example program] for more usages.
//!
//! [`fern::Dispatch::new()`]: struct.Dispatch.html#method.new
//! [`.format(|...| ...)`]: struct.Dispatch.html#method.format
//! [`chrono::Local::now()`]: https://docs.rs/chrono/0.4/chrono/offset/local/struct.Local.html#method.now
//! [`.format(`]: https://docs.rs/chrono/0.4/chrono/datetime/struct.DateTime.html#method.format
//! [`")`]: https://docs.rs/chrono/0.4/chrono/datetime/struct.DateTime.html#method.format
//! [`out.finish(format_args!(...))`]: struct.FormatCallback.html#method.finish
//! [`.level(log::LogLevelFilter::Debug)`]: struct.Dispatch.html#method.level
//! [`Dispatch::chain`]: struct.Dispatch.html#method.chain
//! [`.chain(std::io::stdout())`]: struct.Dispatch.html#method.chain
//! [`Stdout`]: https://doc.rust-lang.org/std/io/struct.Stdout.html
//! [`Stderr`]: https://doc.rust-lang.org/std/io/struct.Stderr.html
//! [`File`]: https://doc.rust-lang.org/std/fs/struct.File.html
//! [`Dispatch`]: struct.Dispatch.html
//! [`.chain(fern::log_file(...)?)`]: struct.Dispatch.html#method.chain
//! [`fern::log_file()`]: fn.log_file.html
//! [`.apply`]: struct.Dispatch.html#method.apply
//! [`format_args!()`]: https://doc.rust-lang.org/std/macro.format_args.html
//! [`println!()`]: https://doc.rust-lang.org/std/macro.println.html
//! [`std::fmt`]: https://doc.rust-lang.org/std/fmt/
//! [`chrono`]: https://github.com/chronotope/chrono
//! [time-and-date docs]: https://docs.rs/chrono/0.4/chrono/index.html#date-and-time
//! [the format specifier docs]: https://docs.rs/chrono/0.4/chrono/format/strftime/index.html#specifiers
//! [`Dispatch` documentation]: struct.Dispatch.html
//! [full example program]: https://github.com/daboross/fern/tree/master/examples/cmd-program.rs
//! [`apply`]: struct.Dispatch.html#method.apply
//! [`log`]: doc.rust-lang.org/log/
extern crate log;

use std::convert::AsRef;
use std::path::Path;
use std::fs::{File, OpenOptions};
use std::{fmt, io};

pub use builders::{Dispatch, Output};
pub use log_impl::FormatCallback;
pub use errors::InitError;

mod builders;
mod log_impl;
mod errors;

/// A type alias for a log formatter.
pub type Formatter = Fn(FormatCallback, &fmt::Arguments, &log::LogRecord) + Sync + Send + 'static;

/// A type alias for a log filter. Returning true means the record should succeed - false means it should fail.
pub type Filter = Fn(&log::LogMetadata) -> bool + Send + Sync + 'static;

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

/// Convenience method for opening a log file with common options.
///
/// Equivalent to:
///
/// ```no_run
/// std::fs::OpenOptions::new()
///     .write(true)
///     .create(true)
///     .append(true)
///     .open("filename")
/// # ;
/// ```
///
/// See [`OpenOptions`] for more information.
///
/// [`OpenOptions`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
#[inline]
pub fn log_file<P: AsRef<Path>>(path: P) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(path)
}
