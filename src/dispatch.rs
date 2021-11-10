use std::{
    borrow::Cow,
    cmp, fmt,
    sync::Arc,
};

use std::collections::HashMap;

use log::Log;

use crate::{Filter, Formatter};
use crate::logger;

/// The base dispatch logger.
///
/// This allows for formatting log records, limiting what records can be passed
/// through, and then dispatching records to other dispatch loggers or output
/// loggers.
///
/// Note that all methods are position-insensitive.
/// `Dispatch::new().format(a).chain(b)` produces the exact same result
/// as `Dispatch::new().chain(b).format(a)`. Given this, it is preferred to put
/// 'format' and other modifiers before 'chain' for the sake of clarity.
///
/// Example usage demonstrating all features:
///
/// ```no_run
/// # // no_run because this creates log files.
/// use std::{fs, io};
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
///             // `io::Stdout`, `io::Stderr` and `io::File` can be directly passed in.
///             .chain(io::stdout()),
///     )
///     .chain(
///         fern::Dispatch::new()
///             // output all messages
///             .level(log::LevelFilter::Trace)
///             // except for hyper, in that case only show info messages
///             .level_for("hyper", log::LevelFilter::Info)
///             // `log_file(x)` equates to
///             // `OpenOptions::new().write(true).append(true).create(true).open(x)`
///             .chain(fern::log_file("persistent-log.log")?)
///             .chain(
///                 fs::OpenOptions::new()
///                     .write(true)
///                     .create(true)
///                     .truncate(true)
///                     .create(true)
///                     .open("/tmp/temp.log")?,
///             ),
///     )
///     .chain(
///         fern::Dispatch::new()
///             .level(log::LevelFilter::Error)
///             .filter(|_meta_data| {
///                 // as an example, randomly reject half of the messages
///                 # /*
///                 rand::random()
///                 # */
///                 # true
///             })
///             .chain(io::stderr()),
///     )
///     // and finally, set as the global logger!
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
    /// The level for all messages without override
    default_level: log::LevelFilter,
    /// Level overrides for specific modules
    levels: Vec<(Cow<'static, str>, log::LevelFilter)>,
    filters: Vec<Box<Filter>>,
}

/// Logger which is usable as an output for multiple other loggers.
///
/// This struct contains a built logger stored in an [`Arc`], and can be
/// safely cloned.
///
/// See [`Dispatch::into_shared`].
///
/// [`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
/// [`Dispatch::into_shared`]: struct.Dispatch.html#method.into_shared
#[derive(Clone)]
pub struct SharedDispatch {
    inner: Arc<DispatchImpl>,
}

impl Log for SharedDispatch {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.inner.log(record);
    }

    fn flush(&self) {
        self.inner.flush();
    }
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

    /// Sets the formatter of this dispatch. The closure should accept a
    /// callback, a message and a log record, and write the resulting
    /// format to the writer.
    ///
    /// The log record is passed for completeness, but the `args()` method of
    /// the record should be ignored, and the [`fmt::Arguments`] given
    /// should be used instead. `record.args()` may be used to retrieve the
    /// _original_ log message, but in order to allow for true log
    /// chaining, formatters should use the given message instead whenever
    /// including the message in the output.
    ///
    /// To avoid all allocation of intermediate results, the formatter is
    /// "completed" by calling a callback, which then calls the rest of the
    /// logging chain with the new formatted message. The callback object keeps
    /// track of if it was called or not via a stack boolean as well, so if
    /// you don't use `out.finish` the log message will continue down
    /// the logger chain unformatted.
    ///
    /// [`fmt::Arguments`]: https://doc.rust-lang.org/std/fmt/struct.Arguments.html
    ///
    /// Example usage:
    ///
    /// ```
    /// fern::Dispatch::new().format(|out, message, record| {
    ///     out.finish(format_args!(
    ///         "[{}][{}] {}",
    ///         record.level(),
    ///         record.target(),
    ///         message
    ///     ))
    /// })
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
    /// All log records which pass all filters will be formatted and then sent
    /// to all child loggers in sequence.
    ///
    /// Children must implement `Into<Output>`. Existing `Log` implementations must
    /// be boxed. Alternatively, [`Self::chain_logger`] can be used.
    ///
    /// Note: If the child logger is also a Dispatch, and cannot accept any log
    /// records, it will be dropped. This only happens if the child either
    /// has no children itself, or has a minimum log level of
    /// [`LevelFilter::Off`].
    ///
    /// [`LevelFilter::Off`]: https://docs.rs/log/0.4/log/enum.LevelFilter.html#variant.Off
    ///
    /// Example usage:
    ///
    /// ```
    /// fern::Dispatch::new().chain(fern::Dispatch::new().chain(std::io::stdout()))
    ///     # .into_log();
    /// ```
    #[inline]
    pub fn chain<T: Into<Output>>(mut self, logger: T) -> Self {
        self.children.push(logger.into().0);
        self
    }

    /// Like [`Self::chain`], but [`Log`] implementations only
    ///
    /// `.chain_logger(logger)` is equivalent to `.chain(Box::new(logger))`.
    /// Note that this method will become obsolete once Rust has trait specialization.
    #[inline]
    pub fn chain_logger<T: Log + 'static>(mut self, logger: T) -> Self {
        self.children.push(OutputInner::Logger(Box::new(logger)));
        self
    }

    /// Sets the overarching level filter for this logger. All messages not
    /// already filtered by something set by [`Dispatch::level_for`] will
    /// be affected.
    ///
    /// All messages filtered will be discarded if less severe than the given
    /// level.
    ///
    /// Default level is [`LevelFilter::Trace`].
    ///
    /// [`Dispatch::level_for`]: #method.level_for
    /// [`LevelFilter::Trace`]: https://docs.rs/log/0.4/log/enum.LevelFilter.html#variant.Trace
    ///
    /// Example usage:
    ///
    /// ```
    /// # fn main() {
    /// fern::Dispatch::new().level(log::LevelFilter::Info)
    ///     # .into_log();
    /// # }
    /// ```
    #[inline]
    pub fn level(mut self, level: log::LevelFilter) -> Self {
        self.default_level = level;
        self
    }

    /// Sets a per-target log level filter. Default target for log messages is
    /// `crate_name::module_name` or
    /// `crate_name` for logs in the crate root. Targets can also be set with
    /// `info!(target: "target-name", ...)`.
    ///
    /// For each log record fern will first try to match the most specific
    /// level_for, and then progressively more general ones until either a
    /// matching level is found, or the default level is used.
    ///
    /// For example, a log for the target `hyper::http::h1` will first test a
    /// level_for for `hyper::http::h1`, then for `hyper::http`, then for
    /// `hyper`, then use the default level.
    ///
    /// Examples:
    ///
    /// A program wants to include a lot of debugging output, but the library
    /// "hyper" is known to work well, so debug output from it should be
    /// excluded:
    ///
    /// ```
    /// # fn main() {
    /// fern::Dispatch::new()
    ///     .level(log::LevelFilter::Trace)
    ///     .level_for("hyper", log::LevelFilter::Info)
    ///     # .into_log();
    /// # }
    /// ```
    ///
    /// A program has a ton of debug output per-module, but there is so much
    /// that debugging more than one module at a time is not very useful.
    /// The command line accepts a list of modules to debug, while keeping the
    /// rest of the program at info level:
    ///
    /// ```
    /// fn setup_logging<T, I>(verbose_modules: T) -> Result<(), fern::InitError>
    /// where
    ///     I: AsRef<str>,
    ///     T: IntoIterator<Item = I>,
    /// {
    ///     let mut config = fern::Dispatch::new().level(log::LevelFilter::Info);
    ///
    ///     for module_name in verbose_modules {
    ///         config = config.level_for(
    ///             format!("my_crate_name::{}", module_name.as_ref()),
    ///             log::LevelFilter::Debug,
    ///         );
    ///     }
    ///
    ///     config.chain(std::io::stdout()).apply()?;
    ///
    ///     Ok(())
    /// }
    /// #
    /// # // we're ok with apply() failing.
    /// # fn main() { let _ = setup_logging(&["hi"]); }
    /// ```
    #[inline]
    pub fn level_for<T: Into<Cow<'static, str>>>(
        mut self,
        module: T,
        level: log::LevelFilter,
    ) -> Self {
        let module = module.into();

        if let Some((index, _)) = self
            .levels
            .iter()
            .enumerate()
            .find(|&(_, &(ref name, _))| name == &module)
        {
            self.levels.remove(index);
        }

        self.levels.push((module, level));
        self
    }

    /// Adds a custom filter which can reject messages passing through this
    /// logger.
    ///
    /// The logger will continue to process log records only if all filters
    /// return `true`.
    ///
    /// [`Dispatch::level`] and [`Dispatch::level_for`] are preferred if
    /// applicable.
    ///
    /// [`Dispatch::level`]: #method.level
    /// [`Dispatch::level_for`]: #method.level_for
    ///
    /// Example usage:
    ///
    /// This sends error level messages to stderr and others to stdout.
    ///
    /// ```
    /// # fn main() {
    /// fern::Dispatch::new()
    ///     .level(log::LevelFilter::Info)
    ///     .chain(
    ///         fern::Dispatch::new()
    ///             .filter(|metadata| {
    ///                 // Reject messages with the `Error` log level.
    ///                 metadata.level() != log::LevelFilter::Error
    ///             })
    ///             .chain(std::io::stderr()),
    ///     )
    ///     .chain(
    ///         fern::Dispatch::new()
    ///             .level(log::LevelFilter::Error)
    ///             .chain(std::io::stdout()),
    ///     )
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

    /// Builds this dispatch and stores it in a clonable structure containing
    /// an [`Arc`].
    ///
    /// Once "shared", the dispatch can be used as an output for multiple other
    /// dispatch loggers.
    ///
    /// Example usage:
    ///
    /// This separates info and warn messages, sending info to stdout + a log
    /// file, and warn to stderr + the same log file. Shared is used so the
    /// program only opens "file.log" once.
    ///
    /// ```no_run
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
    ///         metadata.level() <= log::Level::Info)
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
        SharedDispatch {
            inner: Arc::new(self.into_dispatch()),
        }
    }

    /// Builds this into the actual logger implementation.
    ///
    /// This could probably be refactored, but having everything in one place
    /// is also nice.
    fn into_dispatch(self) -> DispatchImpl {
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
            .filter_map(|child| match child {
                OutputInner::Dispatch(child) => {
                    let child_level = child.max_level;
                    max_child_level = max_child_level.max(child_level);
                    (child_level > log::LevelFilter::Off)
                        .then(|| child)
                        .map(|child| Box::new(child) as Box<dyn Log>)
                }
                OutputInner::DispatchShared(child) => {
                    let child_level = child.inner.max_level;
                    max_child_level = max_child_level.max(child_level);
                    (child_level > log::LevelFilter::Off)
                        .then(|| child)
                        .map(|child| Box::new(child) as Box<dyn Log>)
                }
                OutputInner::Logger(child) => {
                    max_child_level = log::LevelFilter::Trace;
                    Some(child)
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

        DispatchImpl {
            output,
            default_level,
            max_level: real_min,
            levels: levels.into(),
            format,
            filters,
        }
    }

    /// Builds this logger into a `Box<log::Log>` and calculates the minimum
    /// log level needed to have any effect.
    ///
    /// While this method is exposed publicly, [`Dispatch::apply`] is typically
    /// used instead.
    ///
    /// The returned LevelFilter is a calculation for all level filters of this
    /// logger and child loggers, and is the minimum log level needed to
    /// for a record to have any chance of passing through this logger.
    ///
    /// [`Dispatch::apply`]: #method.apply
    ///
    /// Example usage:
    ///
    /// ```
    /// # fn main() {
    /// let (min_level, log) = fern::Dispatch::new()
    ///     .level(log::LevelFilter::Info)
    ///     .chain(std::io::stdout())
    ///     .into_log();
    ///
    /// assert_eq!(min_level, log::LevelFilter::Info);
    /// # }
    /// ```
    pub fn into_log(self) -> (log::LevelFilter, Box<dyn log::Log>) {
        let logger = self.into_dispatch();
        let level = logger.max_level;
        if level == log::LevelFilter::Off {
            (level, Box::new(logger::Null))
        } else {
            (level, Box::new(logger))
        }
    }

    /// Builds this logger and instantiates it as the global [`log`] logger.
    ///
    /// # Errors:
    ///
    /// This function will return an error if a global logger has already been
    /// set to a previous logger.
    ///
    /// [`log`]: https://github.com/rust-lang-nursery/log
    pub fn apply(self) -> Result<(), log::SetLoggerError> {
        let (max_level, log) = self.into_log();

        log::set_boxed_logger(log)?;
        log::set_max_level(max_level);

        Ok(())
    }
}

pub (crate) enum LevelConfiguration {
    JustDefault,
    Minimal(Vec<(Cow<'static, str>, log::LevelFilter)>),
    Many(HashMap<Cow<'static, str>, log::LevelFilter>),
}

pub (crate) struct DispatchImpl {
    pub output: Vec<Box<dyn Log>>,
    /// The level when no level override for a module matches
    pub default_level: log::LevelFilter,
    /// The actual maximum level computed based on the bounds of all children.
    /// The global level must be set to at least this value.
    pub max_level: log::LevelFilter,
    pub levels: LevelConfiguration,
    pub format: Option<Box<Formatter>>,
    pub filters: Vec<Box<Filter>>,
}

/// Callback struct for use within a formatter closure
///
/// Callbacks are used for formatting in order to allow usage of
/// [`std::fmt`]-based formatting without the allocation of the formatted
/// result which would be required to return it.
///
/// Example usage:
///
/// ```
/// fern::Dispatch::new().format(|callback: fern::FormatCallback, message, record| {
///     callback.finish(format_args!("[{}] {}", record.level(), message))
/// })
/// # ;
/// ```
///
/// [`std::fmt`]: https://doc.rust-lang.org/std/fmt/index.html
#[must_use = "format callback must be used for log to process correctly"]
pub struct FormatCallback<'a>(InnerFormatCallback<'a>);

struct InnerFormatCallback<'a>(&'a mut bool, &'a DispatchImpl, &'a log::Record<'a>);

impl From<Vec<(Cow<'static, str>, log::LevelFilter)>> for LevelConfiguration {
    fn from(mut levels: Vec<(Cow<'static, str>, log::LevelFilter)>) -> Self {
        // Benchmarked separately: https://gist.github.com/daboross/976978d8200caf86e02acb6805961195
        // Use Vec if there are fewer than 15 items, HashMap if there are more than 15.
        match levels.len() {
            0 => LevelConfiguration::JustDefault,
            x if x > 15 => LevelConfiguration::Many(levels.into_iter().collect()),
            _ => {
                levels.shrink_to_fit();
                LevelConfiguration::Minimal(levels)
            }
        }
    }
}

impl LevelConfiguration {
    // inline since we use it literally once.
    #[inline]
    fn find_module(&self, module: &str) -> Option<log::LevelFilter> {
        match *self {
            LevelConfiguration::JustDefault => None,
            _ => {
                if let Some(level) = self.find_exact(module) {
                    return Some(level);
                }

                // The manual for loop here lets us just iterate over the module string once
                // while still finding each sub-module. For the module string
                // "hyper::http::h1", this loop will test first "hyper::http"
                // then "hyper".
                let mut last_char_colon = false;

                for (index, ch) in module.char_indices().rev() {
                    if last_char_colon {
                        last_char_colon = false;
                        if ch == ':' {
                            let sub_module = &module[0..index];

                            if let Some(level) = self.find_exact(sub_module) {
                                return Some(level);
                            }
                        }
                    } else if ch == ':' {
                        last_char_colon = true;
                    }
                }

                None
            }
        }
    }

    fn find_exact(&self, module: &str) -> Option<log::LevelFilter> {
        match *self {
            LevelConfiguration::JustDefault => None,
            LevelConfiguration::Minimal(ref levels) => levels
                .iter()
                .find(|&&(ref test_module, _)| test_module == module)
                .map(|&(_, level)| level),
            LevelConfiguration::Many(ref levels) => levels.get(module).cloned(),
        }
    }
}

impl Log for DispatchImpl {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.deep_enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if self.shallow_enabled(record.metadata()) {
            match self.format {
                Some(ref format) => {
                    // flag to ensure the log message is completed even if the formatter doesn't
                    // complete the callback.
                    let mut callback_called_flag = false;

                    (format)(
                        FormatCallback(InnerFormatCallback(
                            &mut callback_called_flag,
                            self,
                            record,
                        )),
                        record.args(),
                        record,
                    );

                    if !callback_called_flag {
                        self.finish_logging(record);
                    }
                }
                None => {
                    self.finish_logging(record);
                }
            }
        }
    }

    fn flush(&self) {
        for log in &self.output {
            log.flush();
        }
    }
}

impl DispatchImpl {
    fn finish_logging(&self, record: &log::Record) {
        for log in &self.output {
            log.log(record);
        }
    }

    /// Check whether this log's filters prevent the given log from happening.
    fn shallow_enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level()
            <= self
                .levels
                .find_module(metadata.target())
                .unwrap_or(self.default_level)
            && self.filters.iter().all(|f| f(metadata))
    }

    /// Check whether a log with the given metadata would eventually end up
    /// outputting something.
    ///
    /// This is recursive, and checks children.
    fn deep_enabled(&self, metadata: &log::Metadata) -> bool {
        self.shallow_enabled(metadata) && self.output.iter().any(|l| l.enabled(metadata))
    }
}

impl<'a> FormatCallback<'a> {
    /// Complete the formatting call that this FormatCallback was created for.
    ///
    /// This will call the rest of the logging chain using the given formatted
    /// message as the new payload message.
    ///
    /// Example usage:
    ///
    /// ```
    /// # fern::Dispatch::new()
    /// # .format(|callback: fern::FormatCallback, message, record| {
    /// callback.finish(format_args!("[{}] {}", record.level(), message))
    /// # })
    /// # .into_log();
    /// ```
    ///
    /// See [`format_args!`].
    ///
    /// [`format_args!`]: https://doc.rust-lang.org/std/macro.format_args.html
    pub fn finish(self, formatted_message: fmt::Arguments) {
        let FormatCallback(InnerFormatCallback(callback_called_flag, dispatch, record)) = self;

        // let the dispatch know that we did in fact get called.
        *callback_called_flag = true;

        // NOTE: This needs to be updated whenever new things are added to
        // `log::Record`.
        let new_record = log::RecordBuilder::new()
            .args(formatted_message)
            .metadata(record.metadata().clone())
            .level(record.level())
            .target(record.target())
            .module_path(record.module_path())
            .file(record.file())
            .line(record.line())
            .build();

        dispatch.finish_logging(&new_record);
    }
}

/// This enum contains various outputs that you can send messages to.
pub(crate) enum OutputInner {
    Dispatch(DispatchImpl),
    DispatchShared(SharedDispatch),
    Logger(Box<dyn Log>),
}

/// Configuration for a logger output.
pub struct Output(pub(crate) OutputInner);

impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Output")
    }
}

impl From<Dispatch> for Output {
    /// Creates an output logger forwarding all messages to the dispatch.
    fn from(log: Dispatch) -> Self {
        Output(OutputInner::Dispatch(log.into_dispatch()))
    }
}

impl From<DispatchImpl> for Output {
    /// Creates an output logger forwarding all messages to the dispatch.
    fn from(log: DispatchImpl) -> Self {
        Output(OutputInner::Dispatch(log))
    }
}

impl From<SharedDispatch> for Output {
    /// Creates an output logger forwarding all messages to the dispatch.
    fn from(log: SharedDispatch) -> Self {
        Output(OutputInner::DispatchShared(log))
    }
}

impl From<Box<dyn Log>> for Output {
    fn from(log: Box<dyn Log>) -> Self {
        Output(OutputInner::Logger(log))
    }
}

impl From<&'static dyn Log> for Output {
    /// Creates an output logger forwarding all messages to the custom logger.
    fn from(log: &'static dyn Log) -> Self {
        struct LogRef(&'static dyn Log);

        impl Log for LogRef {
            fn enabled(&self, metadata: &log::Metadata) -> bool {
                self.0.enabled(metadata)
            }

            fn log(&self, record: &log::Record) {
                self.0.log(record)
            }

            fn flush(&self) {
                self.0.flush()
            }
        }

        Output(OutputInner::Logger(Box::new(LogRef(log))))
    }
}

// impl From<fs::File> for Output {
//     /// Creates an output logger which writes all messages to the file with
//     /// `\n` as the separator.
//     ///
//     /// File writes are buffered and flushed once per log record.
//     fn from(file: fs::File) -> Self {
//         Output(OutputInner::Logger(Box::new(logger::Writer::new(io::BufWriter::new(file)))))
//     }
// }

// impl From<Box<dyn Write + Send>> for Output {
//     /// Creates an output logger which writes all messages to the writer with
//     /// `\n` as the separator.
//     ///
//     /// This does no buffering and it is up to the writer to do buffering as
//     /// needed (eg. wrap it in `BufWriter`). However, flush is called after
//     /// each log record.
//     fn from(writer: Box<dyn Write + Send>) -> Self {
//         Output(OutputInner::Logger(Box::new(logger::Writer::new(writer))))
//     }
// }

// impl From<io::Stdout> for Output {
//     /// Creates an output logger which writes all messages to stdout with the
//     /// given handle and `\n` as the separator.
//     fn from(stream: io::Stdout) -> Self {
//         Output(OutputInner::Logger(Box::new(logger::Writer::from(stream))))
//     }
// }

// impl From<io::Stderr> for Output {
//     /// Creates an output logger which writes all messages to stderr with the
//     /// given handle and `\n` as the separator.
//     fn from(stream: io::Stderr) -> Self {
//         Output(OutputInner::Logger(Box::new(logger::Writer::from(stream))))
//     }
// }

// impl From<Sender<String>> for Output {
//     /// Creates an output logger which writes all messages to the given
//     /// mpsc::Sender with  '\n' as the separator.
//     ///
//     /// All messages sent to the mpsc channel are suffixed with '\n'.
//     fn from(stream: Sender<String>) -> Self {
//         Output(OutputInner::Logger(Box::new(logger::Sender {
//             stream: Mutex::new(stream),
//         })))
//     }
// }

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
            .field("children_count", &self.children.len())
            .field("default_level", &self.default_level)
            .field("levels", &LevelsDebug(&self.levels))
            .field("filters", &FiltersDebug(&self.filters))
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::LevelConfiguration;
    use log::LevelFilter::*;

    #[test]
    fn test_level_config_find_exact_minimal() {
        let config = LevelConfiguration::Minimal(
            vec![("mod1", Info), ("mod2", Debug), ("mod3", Off)]
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        );

        assert_eq!(config.find_exact("mod1"), Some(Info));
        assert_eq!(config.find_exact("mod2"), Some(Debug));
        assert_eq!(config.find_exact("mod3"), Some(Off));
    }

    #[test]
    fn test_level_config_find_exact_many() {
        let config = LevelConfiguration::Many(
            vec![("mod1", Info), ("mod2", Debug), ("mod3", Off)]
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        );

        assert_eq!(config.find_exact("mod1"), Some(Info));
        assert_eq!(config.find_exact("mod2"), Some(Debug));
        assert_eq!(config.find_exact("mod3"), Some(Off));
    }

    #[test]
    fn test_level_config_simple_hierarchy() {
        let config = LevelConfiguration::Minimal(
            vec![("mod1", Info), ("mod2::sub_mod", Debug), ("mod3", Off)]
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        );

        assert_eq!(config.find_module("mod1::sub_mod"), Some(Info));
        assert_eq!(config.find_module("mod2::sub_mod::sub_mod_2"), Some(Debug));
        assert_eq!(config.find_module("mod3::sub_mod::sub_mod_2"), Some(Off));
    }

    #[test]
    fn test_level_config_hierarchy_correct() {
        let config = LevelConfiguration::Minimal(
            vec![
                ("root", Trace),
                ("root::sub1", Debug),
                ("root::sub2", Info),
                // should work with all insertion orders
                ("root::sub2::sub2.3::sub2.4", Error),
                ("root::sub2::sub2.3", Warn),
                ("root::sub3", Off),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
        );

        assert_eq!(config.find_module("root"), Some(Trace));
        assert_eq!(config.find_module("root::other_module"), Some(Trace));

        // We want to ensure that it does pick up most specific level before trying
        // anything more general.
        assert_eq!(config.find_module("root::sub1"), Some(Debug));
        assert_eq!(config.find_module("root::sub1::other_module"), Some(Debug));

        assert_eq!(config.find_module("root::sub2"), Some(Info));
        assert_eq!(config.find_module("root::sub2::other"), Some(Info));

        assert_eq!(config.find_module("root::sub2::sub2.3"), Some(Warn));
        assert_eq!(
            config.find_module("root::sub2::sub2.3::sub2.4"),
            Some(Error)
        );

        assert_eq!(config.find_module("root::sub3"), Some(Off));
        assert_eq!(
            config.find_module("root::sub3::any::children::of::sub3"),
            Some(Off)
        );
    }

    #[test]
    fn test_level_config_similar_names_are_not_same() {
        let config = LevelConfiguration::Minimal(
            vec![("root", Trace), ("rootay", Info)]
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        );

        assert_eq!(config.find_module("root"), Some(Trace));
        assert_eq!(config.find_module("root::sub"), Some(Trace));
        assert_eq!(config.find_module("rooty"), None);
        assert_eq!(config.find_module("rooty::sub"), None);
        assert_eq!(config.find_module("rootay"), Some(Info));
        assert_eq!(config.find_module("rootay::sub"), Some(Info));
    }

    #[test]
    fn test_level_config_single_colon_is_not_double_colon() {
        let config = LevelConfiguration::Minimal(
            vec![
                ("root", Trace),
                ("root::su", Debug),
                ("root::su:b2", Info),
                ("root::sub2", Warn),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
        );

        assert_eq!(config.find_module("root"), Some(Trace));

        assert_eq!(config.find_module("root::su"), Some(Debug));
        assert_eq!(config.find_module("root::su::b2"), Some(Debug));

        assert_eq!(config.find_module("root::su:b2"), Some(Info));
        assert_eq!(config.find_module("root::su:b2::b3"), Some(Info));

        assert_eq!(config.find_module("root::sub2"), Some(Warn));
        assert_eq!(config.find_module("root::sub2::b3"), Some(Warn));
    }

    #[test]
    fn test_level_config_all_chars() {
        let config = LevelConfiguration::Minimal(
            vec![("♲", Trace), ("☸", Debug), ("♲::☸", Info), ("♲::\t", Debug)]
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        );

        assert_eq!(config.find_module("♲"), Some(Trace));
        assert_eq!(config.find_module("♲::other"), Some(Trace));

        assert_eq!(config.find_module("☸"), Some(Debug));
        assert_eq!(config.find_module("☸::any"), Some(Debug));

        assert_eq!(config.find_module("♲::☸"), Some(Info));
        assert_eq!(config.find_module("♲☸"), None);

        assert_eq!(config.find_module("♲::\t"), Some(Debug));
        assert_eq!(config.find_module("♲::\t::\n\n::\t"), Some(Debug));
        assert_eq!(config.find_module("♲::\t\t"), Some(Trace));
    }
}
