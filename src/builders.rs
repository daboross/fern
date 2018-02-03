use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::{cmp, fmt, fs, io};

use log::{self, Log};

use {log_impl, Filter, FormatCallback, Formatter};

#[cfg(feature = "syslog-3")]
use syslog_3;

/// The base dispatch logger.
///
/// This allows for formatting log records, limiting what records can be passed through, and then dispatching records
/// to other dispatch loggers or output loggers.
///
/// Note that all methods are position-insensitive. `Dispatch::new().format(a).chain(b)` produces the exact same result
/// as `Dispatch::new().chain(b).format(a)`. Given this, it is preferred to put 'format' and other modifiers before 'chain'
/// for the sake of clarity.
///
/// Example usage demonstrating all features:
///
/// ```no_run
/// # // no_run because this creates log files.
/// #[macro_use]
/// extern crate log;
/// extern crate fern;
///
/// use std::{io, fs};
///
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// fern::Dispatch::new()
///     .format(|out, message, record| {
///         out.finish(format_args!(
///             "[{}][{}] {}",
///             record.level(),
///             record.target(),
///             message,
///         ))
///     })
///     .chain(
///         fern::Dispatch::new()
///             // by default only accept warn messages
///             .level(log::LevelFilter::Warn)
///             // accept info messages from the current crate too
///             .level_for("my_crate", log::LevelFilter::Info)
///             // `std::io::Stdout`, `std::io::Stderr` and `std::io::File` can be directly passed in.
///             .chain(io::stdout()),
///     )
///     .chain(
///         fern::Dispatch::new()
///             // output all messages
///             .level(log::LevelFilter::Trace)
///             // except for hyper, in that case only show info messages
///             .level_for("hyper", log::LevelFilter::Info)
///             // `log_file(x)` equates to `OpenOptions::new().write(true).append(true).create(true).open(x)`
///             .chain(fern::log_file("persistent-log.log")?)
///             // chain accepts io::File objects, so you can provide your own options too.
///             .chain(fs::OpenOptions::new()
///                 .write(true)
///                 .create(true)
///                 .truncate(true)
///                 .create(true)
///                 .open("/tmp/temp.log")?),
///     )
///     .chain(
///         fern::Dispatch::new()
///             .level(log::LevelFilter::Error)
///             .filter(|_meta_data| {
///                 // let's randomly only send half of the error messages to stderr, that'll be fun!
///                 # /*
///                 rand::random()
///                 # */
///                 # true
///             })
///             .chain(io::stderr()),
///     )
///     // and finally, set as the global logger! This fails if and only if the global logger has already been set.
///     .apply()?;
/// # Ok(())
/// # }
/// #
/// # fn main() { setup_logger().expect("failed to set up logger") }
/// ```
#[must_use = "this is only a logger configuration and must be consumed with into_log() or apply()"]
pub struct Dispatch {
    format: Option<Box<Formatter>>,
    children: Vec<OutputInner>,
    default_level: log::LevelFilter,
    levels: Vec<(Cow<'static, str>, log::LevelFilter)>,
    filters: Vec<Box<Filter>>,
}

/// Logger which is usable as an output for multiple other loggers.
///
/// This struct contains a built logger stored in an [`Arc`], and can be safely cloned.
///
/// See [`Dispatch::into_shared`].
///
/// [`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
/// [`Dispatch::into_shared`]: struct.Dispatch.html#method.into_shared
#[derive(Clone)]
pub struct SharedDispatch {
    inner: Arc<log_impl::Dispatch>,
    min_level: log::LevelFilter,
}

impl Dispatch {
    /// Creates a dispatch, which will initially do nothing.
    #[inline]
    pub fn new() -> Self {
        Dispatch {
            format: None,
            children: Vec::new(),
            default_level: log::LevelFilter::Trace,
            levels: Vec::new(),
            filters: Vec::new(),
        }
    }

    /// Sets the formatter of this dispatch. The closure should accept a callback, a message and a log record, and
    /// write the resulting format to the writer.
    ///
    /// The log record is passed for completeness, but the `args()` method of the record should be ignored, and the
    /// [`fmt::Arguments`] given should be used instead. `record.args()` may be used to retrieve the _original_ log
    /// message, but in order to allow for true log chaining, formatters should use the given message instead whenever
    /// including the message in the output.
    ///
    /// To avoid all allocation of intermediate results, the formatter is "completed" by calling a callback, which then
    /// calls the rest of the logging chain with the new formatted message. The callback object keeps track of if it was
    /// called or not via a stack boolean as well, so if you don't use `out.finish` the log message will continue down
    /// the logger chain unformatted.
    ///
    /// [`fmt::Arguments`]: https://doc.rust-lang.org/std/fmt/struct.Arguments.html
    ///
    /// Example usage:
    ///
    /// ```
    /// fern::Dispatch::new()
    ///     .format(|out, message, record| {
    ///         out.finish(format_args!("[{}][{}] {}", record.level(), record.target(), message))
    ///     })
    ///     # .into_log();
    /// ```
    #[inline]
    pub fn format<F>(mut self, formatter: F) -> Self
    where
        F: Fn(FormatCallback, &fmt::Arguments, &log::Record) + Sync + Send + 'static,
    {
        self.format = Some(Box::new(formatter));
        self
    }

    /// Adds a child to this dispatch.
    ///
    /// All log records which pass all filters will be formatted and then sent to all child loggers in sequence.
    ///
    /// Note: If the child logger is also a Dispatch, and cannot accept any log records, it will be dropped. This
    /// only happens if the child either has no children itself, or has a minimum log level of [`LevelFilter::Off`]
    ///
    /// [`LevelFilter::Off`]: https://doc.rust-lang.org/log/log/enum.LevelFilter.html
    ///
    /// Example usage:
    ///
    /// ```
    /// fern::Dispatch::new()
    ///     .chain(
    ///         fern::Dispatch::new()
    ///             .chain(std::io::stdout())
    ///     )
    ///     # .into_log();
    /// ```
    #[inline]
    pub fn chain<T: Into<Output>>(mut self, logger: T) -> Self {
        self.children.push(logger.into().0);
        self
    }

    /// Sets the overarching level filter for this logger. This will filter all messages which do not fit under another
    /// filter set by `level_for`.
    ///
    /// Default level is [`LevelFilter::Trace`].
    ///
    /// [`LevelFilter::Trace`]: https://doc.rust-lang.org/log/log/enum.LevelFilter.html
    ///
    /// Example usage:
    ///
    /// ```
    /// # extern crate log;
    /// # extern crate fern;
    /// #
    /// # fn main() {
    /// fern::Dispatch::new()
    ///     .level(log::LevelFilter::Info)
    ///     # .into_log();
    /// # }
    /// ```
    #[inline]
    pub fn level(mut self, level: log::LevelFilter) -> Self {
        self.default_level = level;
        self
    }

    /// Sets a per-target log level filter. Default target for log messages is `crate_name::module_name` or
    /// `crate_name` for logs in the crate root. Targets can also be set with `info!(target: "target-name", ...)`.
    ///
    /// For each log record fern will first try to match the most specific level_for, and then progressively more
    /// general ones until either a matching level is found, or the default level is used.
    ///
    /// For example, a log for the target `hyper::http::h1` will first test a level_for for `hyper::http::h1`, then
    /// for `hyper::http`, then for `hyper`, then use the default level.
    ///
    /// Examples:
    ///
    /// A program wants to include a lot of debugging output, but the library "hyper" is known to work well, so
    /// debug output from it should be excluded:
    ///
    /// ```
    /// # extern crate log;
    /// # extern crate fern;
    /// #
    /// # fn main() {
    /// fern::Dispatch::new()
    ///     .level(log::LevelFilter::Trace)
    ///     .level_for("hyper", log::LevelFilter::Info)
    ///     # .into_log();
    /// # }
    /// ```
    ///
    /// A program has a ton of debug output per-module, but there is so much that debugging more than one module at a
    /// time is not very useful. The command line accepts a list of modules to debug, while keeping the rest of the
    /// program at info level:
    ///
    /// ```
    /// # extern crate log;
    /// # extern crate fern;
    /// #
    /// fn setup_logging<T, I>(verbose_modules: T) -> Result<(), fern::InitError>
    ///     where I: AsRef<str>,
    ///           T: IntoIterator<Item = I>
    /// {
    ///     let mut config = fern::Dispatch::new().level(log::LevelFilter::Info);
    ///
    ///     for module_name in verbose_modules {
    ///         config = config.level_for(format!("my_crate_name::{}", module_name.as_ref()),
    ///                                   log::LevelFilter::Debug);
    ///     }
    ///
    ///     config.chain(std::io::stdout()).apply()?;
    ///
    ///     Ok(())
    /// }
    /// #
    /// # fn main() { let _ = setup_logging(&["hi"]); } // we're ok with apply() failing.
    /// ```
    #[inline]
    pub fn level_for<T: Into<Cow<'static, str>>>(mut self, module: T, level: log::LevelFilter) -> Self {
        let module = module.into();

        if let Some((index, _)) = self.levels
            .iter()
            .enumerate()
            .find(|&(_, &(ref name, _))| name == &module)
        {
            self.levels.remove(index);
        }

        self.levels.push((module, level));
        self
    }

    /// Adds a custom filter which can reject messages passing through this logger.
    ///
    /// The logger will continue to process log records only if all filters return `true`.
    ///
    /// [`Dispatch::level`] and [`Dispatch::level_for`] are preferred if applicable.
    ///
    /// [`Dispatch::level`]: #method.level
    /// [`Dispatch::level_for`]: #method.level_for
    ///
    /// Example usage:
    ///
    /// ```
    /// # extern crate log;
    /// # extern crate fern;
    /// #
    /// # fn main() {
    /// fern::Dispatch::new()
    ///     .level(log::LevelFilter::Info)
    ///     .filter(|metadata| {
    ///         // Reject messages with the `Error` log level.
    ///         //
    ///         // This could be useful for sending Error messages to stderr and avoiding duplicate messages in stdout.
    ///         metadata.level() != log::LevelFilter::Error
    ///     })
    ///     # .into_log();
    /// # }
    #[inline]
    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&log::Metadata) -> bool + Send + Sync + 'static,
    {
        self.filters.push(Box::new(filter));
        self
    }

    /// Builds this dispatch and stores it in a clonable structure containing an [`Arc`].
    ///
    /// Once "shared", the dispatch can be used as an output for multiple other dispatch loggers.
    ///
    /// Example usage:
    ///
    /// This separates info and warn messages, sending info to stdout + a log file, and
    /// warn to stderr + the same log file. Shared is used so the program only opens "file.log"
    /// once.
    ///
    /// ```no_run
    /// # extern crate log;
    /// # extern crate fern;
    /// #
    /// # fn setup_logger() -> Result<(), fern::InitError> {
    ///
    /// let file_out = fern::Dispatch::new()
    ///     .chain(fern::log_file("file.log")?)
    ///     .into_shared();
    ///
    /// let info_out = fern::Dispatch::new()
    ///     .level(log::LevelFilter::Debug)
    ///     .filter(|metadata|
    ///         // keep only info and debug (reject warn and error)
    ///         metadata.level() <= log::Level::Info
    ///     )
    ///     .chain(std::io::stdout())
    ///     .chain(file_out.clone());
    ///
    /// let warn_out = fern::Dispatch::new()
    ///     .level(log::LevelFilter::Warn)
    ///     .chain(std::io::stderr())
    ///     .chain(file_out);
    ///
    /// fern::Dispatch::new()
    ///     .chain(info_out)
    ///     .chain(warn_out)
    ///     .apply();
    ///
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { setup_logger().expect("failed to set up logger"); }
    /// ```
    ///
    /// [`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
    pub fn into_shared(self) -> SharedDispatch {
        let (min_level, dispatch) = self.into_dispatch();

        SharedDispatch {
            inner: Arc::new(dispatch),
            min_level: min_level,
        }
    }

    /// Builds this into the actual logger implementation.
    ///
    /// This could probably be refactored, but having everything in one place is also nice.
    fn into_dispatch(self) -> (log::LevelFilter, log_impl::Dispatch) {
        let Dispatch {
            format,
            children,
            default_level,
            levels,
            mut filters,
        } = self;

        let mut max_child_level = log::LevelFilter::Off;

        let output = children
            .into_iter()
            .flat_map(|child| match child {
                OutputInner::Stdout { stream, line_sep } => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::Stdout(log_impl::Stdout {
                        stream: stream,
                        line_sep: line_sep,
                    }))
                }
                OutputInner::Stderr { stream, line_sep } => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::Stderr(log_impl::Stderr {
                        stream: stream,
                        line_sep: line_sep,
                    }))
                }
                OutputInner::File { stream, line_sep } => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::File(log_impl::File {
                        stream: Mutex::new(io::BufWriter::new(stream)),
                        line_sep: line_sep,
                    }))
                }
                OutputInner::Sender { stream, line_sep } => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::Sender(log_impl::Sender {
                        stream: Mutex::new(stream),
                        line_sep: line_sep,
                    }))
                }
                #[cfg(feature = "syslog-3")]
                OutputInner::Syslog(log) => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::Syslog(log_impl::Syslog { inner: log }))
                }
                OutputInner::Dispatch(child_dispatch) => {
                    let (child_level, child) = child_dispatch.into_dispatch();
                    if child_level > log::LevelFilter::Off {
                        max_child_level = cmp::max(max_child_level, child_level);
                        Some(log_impl::Output::Dispatch(child))
                    } else {
                        None
                    }
                }
                OutputInner::SharedDispatch(child_dispatch) => {
                    let SharedDispatch {
                        inner: child,
                        min_level: child_level,
                    } = child_dispatch;

                    if child_level > log::LevelFilter::Off {
                        max_child_level = cmp::max(max_child_level, child_level);
                        Some(log_impl::Output::SharedDispatch(child))
                    } else {
                        None
                    }
                }
                OutputInner::OtherBoxed(child_log) => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::OtherBoxed(child_log))
                }
                OutputInner::OtherStatic(child_log) => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(log_impl::Output::OtherStatic(child_log))
                }
            })
            .collect();

        let min_level = levels
            .iter()
            .map(|t| t.1)
            .max()
            .map_or(default_level, |lvl| cmp::max(lvl, default_level));
        let real_min = cmp::min(min_level, max_child_level);

        filters.shrink_to_fit();

        let dispatch = log_impl::Dispatch {
            output: output,
            default_level: default_level,
            levels: levels.into(),
            format: format,
            filters: filters,
        };

        (real_min, dispatch)
    }

    /// Builds this logger into a `Box<log::Log>` and calculates the minimum log level needed to have any effect.
    ///
    /// While this method is exposed publicly, [`Dispatch::apply`] is typically used instead.
    ///
    /// The returned LevelFilter is a calculation for all level filters of this logger and child loggers, and is the
    /// minimum log level needed to for a record to have any chance of passing through this logger.
    ///
    /// [`Dispatch::apply`]: #method.apply
    ///
    /// Example usage:
    ///
    /// ```
    /// # extern crate log;
    /// # extern crate fern;
    /// #
    /// # fn main() {
    /// let (min_level, log) = fern::Dispatch::new()
    ///     .level(log::LevelFilter::Info)
    ///     .chain(std::io::stdout())
    ///     .into_log();
    ///
    /// assert_eq!(min_level, log::LevelFilter::Info);
    /// # }
    /// ```
    pub fn into_log(self) -> (log::LevelFilter, Box<log::Log>) {
        let (level, logger) = self.into_dispatch();
        if level == log::LevelFilter::Off {
            (level, Box::new(log_impl::Null))
        } else {
            (level, Box::new(logger))
        }
    }

    /// Builds this logger and instantiates it as the global [`log`] logger.
    ///
    /// # Errors:
    ///
    /// This function will return an error if a global logger has already been set to a previous logger.
    ///
    /// [`log`]: https://github.com/rust-lang-nursery/log
    pub fn apply(self) -> Result<(), log::SetLoggerError> {
        let (max_level, log) = self.into_log();

        log::set_boxed_logger(log)?;
        log::set_max_level(max_level);

        Ok(())
    }
}

/// This enum contains various outputs that you can send messages to.
enum OutputInner {
    /// Prints all messages to stdout with `line_sep` separator.
    Stdout {
        stream: io::Stdout,
        line_sep: Cow<'static, str>,
    },
    /// Prints all messages to stderr with `line_sep` separator.
    Stderr {
        stream: io::Stderr,
        line_sep: Cow<'static, str>,
    },
    /// Writes all messages to file with `line_sep` separator.
    File {
        stream: fs::File,
        line_sep: Cow<'static, str>,
    },
    /// Writes all messages to mpst::Sender with `line_sep` separator.
    Sender {
        stream: Sender<String>,
        line_sep: Cow<'static, str>,
    },
    /// Passes all messages to other dispatch.
    Dispatch(Dispatch),
    /// Passes all messages to other dispatch that's shared.
    SharedDispatch(SharedDispatch),
    /// Passes all messages to other logger.
    OtherBoxed(Box<Log>),
    /// Passes all messages to other logger.
    OtherStatic(&'static Log),
    #[cfg(feature = "syslog-3")]
    Syslog(syslog_3::Logger),
}

/// Configuration for a logger output.
pub struct Output(OutputInner);

impl From<Dispatch> for Output {
    /// Creates an output logger forwarding all messages to the dispatch.
    fn from(log: Dispatch) -> Self {
        Output(OutputInner::Dispatch(log))
    }
}

impl From<SharedDispatch> for Output {
    /// Creates an output logger forwarding all messages to the dispatch.
    fn from(log: SharedDispatch) -> Self {
        Output(OutputInner::SharedDispatch(log))
    }
}

impl From<Box<Log>> for Output {
    /// Creates an output logger forwarding all messages to the custom logger.
    fn from(log: Box<Log>) -> Self {
        Output(OutputInner::OtherBoxed(log))
    }
}

impl From<&'static Log> for Output {
    /// Creates an output logger forwarding all messages to the custom logger.
    fn from(log: &'static Log) -> Self {
        Output(OutputInner::OtherStatic(log))
    }
}

impl From<fs::File> for Output {
    /// Creates an output logger which writes all messages to the file with `\n` as the separator.
    ///
    /// File writes are buffered and flushed once per log record.
    fn from(file: fs::File) -> Self {
        Output(OutputInner::File {
            stream: file,
            line_sep: "\n".into(),
        })
    }
}

impl From<io::Stdout> for Output {
    /// Creates an output logger which writes all messages to stdout with the given handle and `\n` as the separator.
    fn from(stream: io::Stdout) -> Self {
        Output(OutputInner::Stdout {
            stream: stream,
            line_sep: "\n".into(),
        })
    }
}

impl From<io::Stderr> for Output {
    /// Creates an output logger which writes all messages to stderr with the given handle and `\n` as the separator.
    fn from(stream: io::Stderr) -> Self {
        Output(OutputInner::Stderr {
            stream: stream,
            line_sep: "\n".into(),
        })
    }
}

impl From<Sender<String>> for Output {
    /// Creates an output logger which writes all messages to the given mpsc::Sender with  '\n' as the separator.
    ///
    /// All messages sent to the mpsc channel are suffixed with '\n'.
    fn from(stream: Sender<String>) -> Self {
        Output(OutputInner::Sender {
            stream: stream,
            line_sep: "\n".into(),
        })
    }
}

#[cfg(feature = "syslog-3")]
impl From<syslog_3::Logger> for Output {
    /// Creates an output logger which writes all messages to the given syslog output.
    ///
    /// Log levels are translated trace => debug, debug => debug, info => informational, warn => warning, and
    /// error => error.
    fn from(log: syslog_3::Logger) -> Self {
        Output(OutputInner::Syslog(log))
    }
}

#[cfg(feature = "syslog-3")]
impl From<Box<syslog_3::Logger>> for Output {
    /// Creates an output logger which writes all messages to the given syslog output.
    ///
    /// Log levels are translated trace => debug, debug => debug, info => informational, warn => warning, and
    /// error => error.
    ///
    /// Note that while this takes a Box<Logger> for convenience (syslog methods return Boxes), it will be
    /// immediately unboxed upon storage in the configuration structure. This will create a configuration
    /// identical to that created by passing a raw `syslog::Logger`.
    fn from(log: Box<syslog_3::Logger>) -> Self {
        Output(OutputInner::Syslog(*log))
    }
}

impl Output {
    /// Returns a file logger using a custom separator.
    ///
    /// If the default separator of `\n` is acceptable, an [`fs::File`] instance can be passed into
    /// [`Dispatch::chain`] directly.
    ///
    /// ```no_run
    /// # fn setup_logger() -> Result<(), fern::InitError> {
    /// fern::Dispatch::new()
    ///     .chain(std::fs::File::create("log")?)
    ///     # .into_log();
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { setup_logger().expect("failed to set up logger"); }
    /// ```
    ///
    /// ```no_run
    /// # fn setup_logger() -> Result<(), fern::InitError> {
    /// fern::Dispatch::new()
    ///     .chain(fern::log_file("log")?)
    ///     # .into_log();
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { setup_logger().expect("failed to set up logger"); }
    ///
    /// ```
    ///
    /// Example usage (using [`fern::log_file`]):
    ///
    /// ```no_run
    /// # fn setup_logger() -> Result<(), fern::InitError> {
    /// fern::Dispatch::new()
    ///     .chain(fern::Output::file(fern::log_file("log")?, "\r\n"))
    ///     # .into_log();
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { setup_logger().expect("failed to set up logger"); }
    /// ```
    ///
    /// [`fs::File`]: https://doc.rust-lang.org/std/fs/struct.File.html
    /// [`Dispatch::chain`]: struct.Dispatch.html#method.chain
    /// [`fern::log_file`]: fn.log_file.html
    pub fn file<T: Into<Cow<'static, str>>>(file: fs::File, line_sep: T) -> Self {
        Output(OutputInner::File {
            stream: file,
            line_sep: line_sep.into(),
        })
    }

    /// Returns an stdout logger using a custom separator.
    ///
    /// If the default separator of `\n` is acceptable, an `io::Stdout` instance can be passed into
    /// `Dispatch::chain()` directly.
    ///
    /// ```
    /// fern::Dispatch::new()
    ///     .chain(std::io::stdout())
    ///     # .into_log();
    /// ```
    pub fn stdout<T: Into<Cow<'static, str>>>(line_sep: T) -> Self {
        Output(OutputInner::Stdout {
            stream: io::stdout(),
            line_sep: line_sep.into(),
        })
    }

    /// Returns an stderr logger using a custom separator.
    ///
    /// If the default separator of `\n` is acceptable, an `io::Stderr` instance can be passed into
    /// `Dispatch::chain()` directly.
    ///
    /// ```
    /// fern::Dispatch::new()
    ///     .chain(std::io::stderr())
    ///     # .into_log();
    /// ```
    pub fn stderr<T: Into<Cow<'static, str>>>(line_sep: T) -> Self {
        Output(OutputInner::Stderr {
            stream: io::stderr(),
            line_sep: line_sep.into(),
        })
    }

    /// Returns a mpsc::Sender logger using a custom separator.
    ///
    /// If the default separator of `\n` is acceptable, an `mpsc::Sender<String>` instance can be passed into
    /// `Dispatch::chain()` directly.
    ///
    /// Each log message will be suffixed with the separator, then sent as a single String to the given sender.
    ///
    /// ```
    /// use std::sync::mpsc::channel;
    ///
    /// let (tx, rx) = channel();
    /// fern::Dispatch::new()
    ///     .chain(tx)
    ///     # .into_log();
    /// ```
    pub fn sender<T: Into<Cow<'static, str>>>(sender: Sender<String>, line_sep: T) -> Self {
        Output(OutputInner::Sender {
            stream: sender,
            line_sep: line_sep.into(),
        })
    }
}

impl Default for Dispatch {
    /// Returns a logger configuration that does nothing with log records.
    ///
    /// Equivalent to [`Dispatch::new`].
    ///
    /// [`Dispatch::new`]: #method.new
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct LevelsDebug<'a>(&'a [(Cow<'static, str>, log::LevelFilter)]);
        impl<'a> fmt::Debug for LevelsDebug<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.debug_map()
                    .entries(self.0.iter().map(|t| (t.0.as_ref(), t.1)))
                    .finish()
            }
        }
        struct FiltersDebug<'a>(&'a [Box<Filter>]);
        impl<'a> fmt::Debug for FiltersDebug<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.debug_list()
                    .entries(self.0.iter().map(|_| "<filter closure>"))
                    .finish()
            }
        }
        f.debug_struct("Dispatch")
            .field(
                "format",
                &self.format.as_ref().map(|_| "<formatter closure>"),
            )
            .field("children", &self.children)
            .field("default_level", &self.default_level)
            .field("levels", &LevelsDebug(&self.levels))
            .field("filters", &FiltersDebug(&self.filters))
            .finish()
    }
}

impl fmt::Debug for OutputInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OutputInner::Stdout {
                ref stream,
                ref line_sep,
            } => f.debug_struct("Output::Stdout")
                .field("stream", stream)
                .field("line_sep", line_sep)
                .finish(),
            OutputInner::Stderr {
                ref stream,
                ref line_sep,
            } => f.debug_struct("Output::Stderr")
                .field("stream", stream)
                .field("line_sep", line_sep)
                .finish(),
            OutputInner::File {
                ref stream,
                ref line_sep,
            } => f.debug_struct("Output::File")
                .field("stream", stream)
                .field("line_sep", line_sep)
                .finish(),
            OutputInner::Sender {
                ref stream,
                ref line_sep,
            } => f.debug_struct("Output::Sender")
                .field("stream", stream)
                .field("line_sep", line_sep)
                .finish(),
            #[cfg(feature = "syslog-3")]
            OutputInner::Syslog(_) => f.debug_tuple("Output::Syslog")
                .field(&"<unprintable syslog::Logger>")
                .finish(),
            OutputInner::Dispatch(ref dispatch) => f.debug_tuple("Output::Dispatch").field(dispatch).finish(),
            OutputInner::SharedDispatch(_) => f.debug_tuple("Output::SharedDispatch")
                .field(&"<built Dispatch logger>")
                .finish(),
            OutputInner::OtherBoxed { .. } => f.debug_tuple("Output::OtherBoxed")
                .field(&"<boxed logger>")
                .finish(),
            OutputInner::OtherStatic { .. } => f.debug_tuple("Output::OtherStatic")
                .field(&"<boxed logger>")
                .finish(),
        }
    }
}

impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
