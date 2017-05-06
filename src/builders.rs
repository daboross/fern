use std::borrow::Cow;
use std::sync::Mutex;
use std::{io, fs, cmp, fmt};

use log;

use {log_impl, FernLog, Formatter, Filter};

/// The base dispatch logger.
///
/// This allows for formatting log records, limiting what records can be passed through, and then dispatching records
/// to other dispatch loggers or output loggers.
///
/// Note that all methods are position-insensitive. `Dispatch::new().format(a).chain(b)` produces the exact same result
/// as `Dispatch::new().chain(b).format(a)`. Even with this, the first syntax is preferred for clarity's sake.
///
/// Example usage demonstrating all features:
///
/// ```no_run
/// # // no_run because this creates log files.
/// extern crate log;
/// extern crate fern;
///
/// use std::io;
/// use std::fs;
///
/// fern::Dispatch::new()
///     .format(|out, message, record| {
///         use std::fmt::Write;
///
///         write!(out, "[{}][{}] {}", record.level(), record.target(), message)
///     })
///     .chain(
///         fern::Dispatch::new()
///             // by default only accept warn messages
///             .level(log::LogLevelFilter::Warn)
///             // accept info messages from the current crate too
///             .level_for("my_crate", log::LogLevelFilter::Info)
///             // `std::io::Stdout`, `std::io::Stderr` and `std::io::File` can be directly passed in.
///             .chain(io::stdout())
///     )
///     .chain(
///         fern::Dispatch::new()
///             // output all messages
///             .level(log::LogLevelFilter::Trace)
///             // except for hyper, in that case only show info messages
///             .level_for("hyper", log::LogLevelFilter::Info)
///             // in a real application, you'd probably want to handle this error.
///             // `log_file(x)` equates to `OpenOptions::new().write(true).append(true).create(true).open(x)`
///             .chain(fern::log_file("persistent-log.log").expect("failed to open log file"))
///             // chain accepts io::File objects, so you can provide your own options too.
///             .chain(fs::OpenOptions::new()
///                 .write(true)
///                 .create(true)
///                 .truncate(true)
///                 .create(true)
///                 .open("/tmp/temp.log")
///                 .expect("failed to open log file"))
///     )
///     .chain(
///         fern::Dispatch::new()
///             .level(log::LogLevelFilter::Error)
///             .filter(|_meta_data| {
///                 // let's randomly only send half of the error messages to stderr, that'll be fun!
///                 # /*
///                 rand::random()
///                 # */
///                 # true
///             })
///             .chain(io::stderr())
///     )
///     // and finally, set as the global logger!
///     .set_global()
///     // this can also fail (only happens if global logger has been set before.)
///     .unwrap()
/// ```
pub struct Dispatch {
    format: Option<Box<Formatter>>,
    children: Vec<OutputInner>,
    default_level: log::LogLevelFilter,
    levels: Vec<(Cow<'static, str>, log::LogLevelFilter)>,
    filters: Vec<Box<Filter>>,
}


impl Dispatch {
    /// Creates a dispatch, which will initially do nothing.
    #[inline]
    pub fn new() -> Self {
        Dispatch {
            format: None,
            children: Vec::new(),
            default_level: log::LogLevelFilter::Trace,
            levels: Vec::new(),
            filters: Vec::new(),
        }
    }

    /// Sets the formatter of this dispatch. The closure should accept a writer, a message and a log record, and
    /// write the resulting format to the writer.
    ///
    /// The log record is passed for completeness, but the 'args' method of the record should be ignored, and the
    /// fmt::Arguments given should be used instead. `record.args()` may be used to retrieve the _original_ log message,
    /// but in order to allow for true log chaining, formatters should use the given message instead whenever including
    /// the message in the output.
    ///
    /// In order to avoid allocation of intermediate results, the closure will be called once per endpoint per log
    /// record. For instance, if you have messages going to both the console and to a file, the formatter will be
    /// called twice per log record. For this reason it is recommended not to do any heavy calculations within the
    /// formatter.
    ///
    /// Example usage:
    ///
    /// ```
    /// fern::Dispatch::new()
    ///     // ...
    ///     .format(|writer, message, record| {
    ///         use std::fmt::Write;
    ///
    ///         write!(writer, "[{}][{}] {}", record.target(), record.level(), message)
    ///     })
    ///     // ...
    ///     # /*
    ///     .set_global();
    ///     # */
    ///     # .into_log();
    /// ```
    #[inline]
    pub fn format<F>(mut self, formatter: F) -> Self
        where F: Fn(&mut fmt::Write, &fmt::Arguments, &log::LogRecord) -> fmt::Result + Sync + Send + 'static
    {
        self.format = Some(Box::new(formatter));
        self
    }

    /// Adds an output to this logger. Any log record which passes through all filters will be sent through the
    /// formatter (if any) then be passed to all chained loggers.
    #[inline]
    pub fn chain<T: Into<Output>>(mut self, logger: T) -> Self {
        self.children.push(logger.into().0);
        self
    }

    /// Sets the overarching level filter for this logger. This will filter all messages which do not fit under another
    /// filter set by `level_for`.
    ///
    /// Default level is `LogLevelFilter::Trace`.
    #[inline]
    pub fn level(mut self, level: log::LogLevelFilter) -> Self {
        self.default_level = level;
        self
    }

    /// Sets the level filter for a specific module, overwriting any previous filter for that module name. This will
    /// overwrite the level set by `level` for this module, if any.
    #[inline]
    pub fn level_for<T: Into<Cow<'static, str>>>(mut self, module: T, level: log::LogLevelFilter) -> Self {
        let module = module.into();

        if let Some((index, _)) = self.levels.iter().enumerate().find(|&(_, &(ref name, _))| name == &module) {
            self.levels.remove(index);
        }

        self.levels.push((module, level));
        self
    }

    /// Adds a filter for all log records passing through this logger. The filter will be called if the log record is
    /// successfully enabled by all other measures (level set or level for the specific module set).
    ///
    /// Note that setting `level` and/or `level_for` is preferred to this method for loggers that aren't top level, as
    /// those can be propagated up to the top level crate to avoid processing messages that won't be consumed. Using
    /// `filter` forces log records to go through all loggers up to the one with filters in it.
    ///
    /// Any record for which the filter returns `false` will be dropped and not sent to any chained loggers.
    #[inline]
    pub fn filter<F>(mut self, filter: F) -> Self
        where F: Fn(&log::LogMetadata) -> bool + Send + Sync + 'static
    {
        self.filters.push(Box::new(filter));
        self
    }

    /// Internal build method. This does the grunt of the work, and probably could be refactored into multiple methods
    /// in the future.
    fn into_dispatch(self) -> (log::LogLevelFilter, log_impl::Dispatch) {
        let Dispatch { format, children, default_level, levels, mut filters } = self;

        let mut max_child_level = log::LogLevelFilter::Off;

        let output = children.into_iter()
            .flat_map(|child| match child {
                OutputInner::Stdout { stream, line_sep } => {
                    max_child_level = log::LogLevelFilter::Trace;
                    Some(log_impl::Output::Stdout(log_impl::Stdout {
                        stream: stream,
                        line_sep: line_sep,
                    }))
                }
                OutputInner::Stderr { stream, line_sep } => {
                    max_child_level = log::LogLevelFilter::Trace;
                    Some(log_impl::Output::Stderr(log_impl::Stderr {
                        stream: stream,
                        line_sep: line_sep,
                    }))
                }
                OutputInner::File { stream, line_sep } => {
                    max_child_level = log::LogLevelFilter::Trace;
                    Some(log_impl::Output::File(log_impl::File {
                        stream: Mutex::new(io::BufWriter::new(stream)),
                        line_sep: line_sep,
                    }))
                }
                OutputInner::Dispatch(child_dispatch) => {
                    let (child_level, child) = child_dispatch.into_dispatch();
                    if child_level > log::LogLevelFilter::Off {
                        max_child_level = cmp::max(max_child_level, child_level);
                        Some(log_impl::Output::Dispatch(child))
                    } else {
                        None
                    }
                }
                OutputInner::Other(child_log) => {
                    max_child_level = log::LogLevelFilter::Trace;
                    Some(log_impl::Output::Other(child_log))
                }
            })
            .collect();

        let min_level = levels.iter().map(|t| t.1).max().map_or(default_level, |lvl| cmp::max(lvl, default_level));
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

    /// Builds this logger into a `Box<log::Log>` and calculates the maximum level that this logger will accept. This is
    /// typically only called internally, though it can be used publicly for interacting with other log crates.
    ///
    /// The returned LogLevelFilter is a calculation from this logger and all chained loggers, and will be the minimum
    /// level which might have some impact - this takes into account all per-module and global levels of all children.
    ///
    /// The recommended endpoint for a logger builder is `set_global()`, which consumes this configuration and sets it
    /// as the global logger for the `log` crate.
    pub fn into_log(self) -> (log::LogLevelFilter, Box<log::Log>) {
        let (level, logger) = self.into_dispatch();
        if level == log::LogLevelFilter::Off {
            (level, Box::new(log_impl::Null))
        } else {
            (level, Box::new(logger))
        }
    }

    /// Builds this logger and sets it as the `log` crates global logger. This method will fail if a logger has already
    /// been set for the `log` crate or if an IO error occurs opening any target files or streams.
    pub fn set_global(self) -> Result<(), log::SetLoggerError> {
        let (max_level, log) = self.into_log();

        log::set_logger(|max_level_storage| {
            max_level_storage.set(max_level);

            log
        })
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
    /// Passes all messages to other dispatch.
    Dispatch(Dispatch),
    /// Passes all messages to other logger.
    Other(Box<FernLog>),
}

/// Configuration for a logger output.
pub struct Output(OutputInner);

impl From<Dispatch> for Output {
    /// Creates an output logger forwarding all messages to the dispatch.
    fn from(log: Dispatch) -> Self {
        Output(OutputInner::Dispatch(log))
    }
}

impl From<Box<FernLog>> for Output {
    /// Creates an output logger forwarding all messages to the custom logger.
    fn from(log: Box<FernLog>) -> Self {
        Output(OutputInner::Other(log))
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

impl Output {
    /// Returns a file logger using a custom separator.
    ///
    /// If the default separator of `\n` is acceptable, an `fs::File` instance can be passed into
    /// `Dispatch::chain()` directly.
    ///
    /// ```
    /// Dispatch::new().chain(std::fs::File::create("log").unwrap())
    /// # ;
    /// ```
    ///
    /// ```
    /// Dispatch::new().chain(fern::log_file("log").unwrap())
    /// # ;
    /// ```
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
    /// Dispatch::new().chain(std::io::stdout())
    /// # ;
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
    /// Dispatch::new().chain(std::io::stderr())
    /// # ;
    /// ```
    pub fn stderr<T: Into<Cow<'static, str>>>(line_sep: T) -> Self {
        Output(OutputInner::Stderr {
            stream: io::stderr(),
            line_sep: line_sep.into(),
        })
    }
}
