use std::{
    borrow::Cow,
    fs,
    io::{self, BufWriter, Write},
    sync::{mpsc, Mutex},
};

#[cfg(feature = "date-based")]
use std::{
    ffi::OsString,
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use log::{self, Log};

use crate::{*, dispatch::{Output, OutputInner}};

pub trait CustomLineSep {
    fn line_sep(self, line_sep: Cow<'static, str>) -> Self;
}

/// Returns an stdout logger using a custom separator.
///
/// If the default separator of `\n` is acceptable, an `io::Stdout`
/// instance can be passed into `Dispatch::chain()` directly.
///
/// ```
/// fern::Dispatch::new().chain(std::io::stdout())
///     # .into_log();
/// ```
///
/// Example usage:
///
/// ```
/// fern::Dispatch::new()
///     // some unix tools use null bytes as message terminators so
///     // newlines in messages can be treated differently.
///     .chain(fern::Output::stdout("\0"))
///     # .into_log();
/// ```
pub fn stdout() -> impl Log + CustomLineSep + Into<Output> {
    Writer::new(io::stdout())
}

/// Returns an stderr logger using a custom separator.
///
/// If the default separator of `\n` is acceptable, an `io::Stderr`
/// instance can be passed into `Dispatch::chain()` directly.
///
/// ```
/// fern::Dispatch::new().chain(std::io::stderr())
///     # .into_log();
/// ```
///
/// Example usage:
///
/// ```
/// fern::Dispatch::new().chain(fern::Output::stderr("\n\n\n"))
///     # .into_log();
/// ```
pub fn stderr() -> impl Log + CustomLineSep + Into<Output> {
    Writer::new(io::stderr())
}

/// Returns a file logger using a custom separator.
///
/// If the default separator of `\n` is acceptable, an [`fs::File`]
/// instance can be passed into [`Dispatch::chain`] directly.
///
/// ```no_run
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// fern::Dispatch::new().chain(std::fs::File::create("log")?)
///     # .into_log();
/// # Ok(())
/// # }
/// #
/// # fn main() { setup_logger().expect("failed to set up logger"); }
/// ```
///
/// ```no_run
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// fern::Dispatch::new().chain(fern::log_file("log")?)
///     # .into_log();
/// # Ok(())
/// # }
/// #
/// # fn main() { setup_logger().expect("failed to set up logger"); }
/// ```
///
/// Example usage (using [`fern::log_file`]):
///
/// ```no_run
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// fern::Dispatch::new().chain(fern::Output::file(fern::log_file("log")?, "\r\n"))
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
pub fn file(path: impl AsRef<Path>) -> std::io::Result<impl Log + CustomLineSep + Into<Output>> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(path)
        .map(std::io::BufWriter::new)
        .map(Writer::new)
}

/// Returns a logger using arbitrary write object and custom separator.
///
/// If the default separator of `\n` is acceptable, an `Box<Write + Send>`
/// instance can be passed into [`Dispatch::chain`] directly.
///
/// ```no_run
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// // Anything implementing 'Write' works.
/// let mut writer = std::io::Cursor::new(Vec::<u8>::new());
///
/// fern::Dispatch::new()
///     // as long as we explicitly cast into a type-erased Box
///     .chain(Box::new(writer) as Box<std::io::Write + Send>)
///     # .into_log();
/// #     Ok(())
/// # }
/// #
/// # fn main() { setup_logger().expect("failed to set up logger"); }
/// ```
///
/// Example usage:
///
/// ```no_run
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// let writer = Box::new(std::io::Cursor::new(Vec::<u8>::new()));
///
/// fern::Dispatch::new().chain(fern::Output::writer(writer, "\r\n"))
///     # .into_log();
/// #     Ok(())
/// # }
/// #
/// # fn main() { setup_logger().expect("failed to set up logger"); }
/// ```
///
/// [`Dispatch::chain`]: struct.Dispatch.html#method.chain
pub fn writer<W: Write + Send + 'static>(writer: W) -> impl Log + CustomLineSep + Into<Output> {
    Writer::new(writer)
}

/// Returns a reopenable logger, i.e., handling SIGHUP.
///
/// If the default separator of `\n` is acceptable, a `Reopen`
/// instance can be passed into [`Dispatch::chain`] directly.
///
/// This function is not available on Windows, and it requires the `reopen-03`
/// feature to be enabled.
///
/// ```no_run
/// use std::fs::OpenOptions;
/// # fn setup_logger() -> Result<(), fern::InitError> {
/// let reopenable = reopen::Reopen::new(Box::new(|| {
///     OpenOptions::new()
///         .create(true)
///         .write(true)
///         .append(true)
///         .open("/tmp/output.log")
/// }))
/// .unwrap();
///
/// fern::Dispatch::new().chain(fern::Output::reopen(reopenable, "\n"))
///     # .into_log();
/// #     Ok(())
/// # }
/// #
/// # fn main() { setup_logger().expect("failed to set up logger"); }
/// ```
/// [`Dispatch::chain`]: struct.Dispatch.html#method.chain

/// Convenience method for opening a re-openable log file with common options.
///
/// The file opening is equivalent to:
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
///
/// This function is not available on Windows, and it requires the `reopen-03`
/// feature to be enabled.
#[cfg(all(not(windows), feature = "reopen-03"))]
#[allow(deprecated)]
pub fn reopen(path: impl Into<PathBuf>, signal: Option<libc::c_int>) -> io::Result<impl Log + CustomLineSep + Into<Output>> {
    let path = path.into();
    let reopen = reopen::Reopen::new(Box::new(move || {
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&path)
    }))?;

    if let Some(s) = signal {
        reopen.handle().register_signal(s)?;
    }
    Ok(Writer::new(reopen))
}

/// Returns a mpsc::Sender logger using a custom separator.
///
/// If the default separator of `\n` is acceptable, an
/// `mpsc::Sender<String>` instance can be passed into `Dispatch::
/// chain()` directly.
///
/// Each log message will be suffixed with the separator, then sent as a
/// single String to the given sender.
///
/// ```
/// use std::sync::mpsc::channel;
///
/// let (tx, rx) = channel();
/// fern::Dispatch::new().chain(tx)
///     # .into_log();
/// ```
pub fn sender(sender: mpsc::Sender<String>) -> impl Log + Into<Output> {
    struct Sender {
        stream: Mutex<mpsc::Sender<String>>,
    }

    impl Log for Sender {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
    
        fn log(&self, record: &log::Record) {
            fallback_on_error(record, |record| {
                let msg = format!("{}", record.args());
                self.stream
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .send(msg)?;
                Ok(())
            });
        }
    
        fn flush(&self) {}
    }

    impl From<Sender> for Output {
        fn from(log: Sender) -> Self {
            Output(OutputInner::Logger(Box::new(log)))
        }
    }

    Sender { stream: Mutex::new(sender), }
}

/// Returns a logger which simply calls the given function with each
/// message.
///
/// The function will be called inline in the thread the log occurs on.
///
/// Example usage:
///
/// ```
/// fern::Dispatch::new().chain(fern::Output::call(|record| {
///     // this is mundane, but you can do anything here.
///     println!("{}", record.args());
/// }))
///     # .into_log();
/// ```
pub fn call<F>(func: F) -> impl Log + Into<Output>
where
    F: Fn(&log::Record) + Sync + Send + 'static,
{
    struct CallShim<F>(F);

    impl<F> log::Log for CallShim<F>
    where
        F: Fn(&log::Record) + Sync + Send + 'static,
    {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            (self.0)(record)
        }
        fn flush(&self) {}
    }

    impl <F: Fn(&log::Record) + Sync + Send + 'static> From<CallShim<F>> for Output {
        fn from(log: CallShim<F>) -> Output {
            Output(OutputInner::Logger(Box::new(log)))
        }
    }

    CallShim(func)
}

struct Writer<W: Write + Send> {
    stream: Mutex<W>,
    line_sep: Cow<'static, str>,
}

impl <W: Write + Send> Writer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            stream: Mutex::new(writer),
            line_sep: "\n".into(),
        }
    }
}

impl <W: Write + Send> CustomLineSep for Writer<W> {
    fn line_sep(mut self, line_sep: Cow<'static, str>) -> Self {
        self.line_sep = line_sep;
        self
    }
}

// impl From<io::Stdout> for Writer<io::Stdout> {
//     fn from(stream: io::Stdout) -> Self {
//         Self::new(stream)
//     }
// }

// impl From<io::Stderr> for Writer<io::Stderr> {
//     fn from(stream: io::Stderr) -> Self {
//         Self::new(stream)
//     }
// }

// #[cfg(all(not(windows), feature = "reopen-03"))]
// impl From<reopen::Reopen<fs::File>> for Writer<reopen::Reopen<fs::File>> {
//     fn from(stream: reopen::Reopen<fs::File>) -> Self {
//         Self::new(stream)
//     }
// }

impl <W: Write + Send> Log for Writer<W> {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            if cfg!(feature = "meta-logging-in-format") {
                // Formatting first prevents deadlocks on file-logging,
                // when the process of formatting itself is logged.
                // note: this is only ever needed if some Debug, Display, or other
                // formatting trait itself is logging.
                let msg = format!("{}{}", record.args(), self.line_sep);

                let mut writer = self.stream.lock().unwrap_or_else(|e| e.into_inner());

                write!(writer, "{}", msg)?;

                writer.flush()?;
            } else {
                let mut writer = self.stream.lock().unwrap_or_else(|e| e.into_inner());

                write!(writer, "{}{}", record.args(), self.line_sep)?;

                writer.flush()?;
            }
            Ok(())
        });
    }

    fn flush(&self) {
        let _ = self.stream.lock().unwrap_or_else(|e| e.into_inner())
            .flush();
    }
}

impl <W: Write + Send + 'static> From<Writer<W>> for Output {
    fn from(log: Writer<W>) -> Self {
        Output(OutputInner::Logger(Box::new(log)))
    }
}

/// Writes all messages to the reopen::Reopen file with `line_sep`
/// separator.

/// Logger which will panic whenever anything is logged. The panic
/// will be exactly the message of the log.
///
/// `Panic` is useful primarily as a secondary logger, filtered by warning or
/// error.
///
/// # Examples
///
/// This configuration will output all messages to stdout and panic if an Error
/// message is sent.
///
/// ```
/// fern::Dispatch::new()
///     // format, etc.
///     .chain(std::io::stdout())
///     .chain(
///         fern::Dispatch::new()
///             .level(log::LevelFilter::Error)
///             .chain(fern::Panic),
///     )
///     # /*
///     .apply()?;
///     # */ .into_log();
/// ```
///
/// This sets up a "panic on warn+" logger, and ignores errors so it can be
/// called multiple times.
///
/// This might be useful in test setup, for example, to disallow warn-level
/// messages.
///
/// ```no_run
/// fn setup_panic_logging() {
///     fern::Dispatch::new()
///         .level(log::LevelFilter::Warn)
///         .chain(fern::Panic)
///         .apply()
///         // ignore errors from setting up logging twice
///         .ok();
/// }
/// ```
pub struct Panic;

impl Log for Panic {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        panic!("{}", record.args());
    }

    fn flush(&self) {}
}

impl From<Panic> for Output {
    /// Creates an output logger which will panic with message text for all
    /// messages.
    fn from(log: Panic) -> Self {
        Output(OutputInner::Logger(Box::new(log)))
    }
}

pub struct Null;

impl Log for Null {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _: &log::Record) {}

    fn flush(&self) {}
}

impl From<Null> for Output {
    fn from(log: Null) -> Self {
        Output(OutputInner::Logger(Box::new(log)))
    }
}

/// This is used to generate log file suffixed based on date, hour, and minute.
///
/// The log file will be rotated automatically when the date changes.
#[derive(Debug)]
#[cfg(feature = "date-based")]
pub struct DateBased {
    pub(crate) file_prefix: PathBuf,
    pub(crate) file_suffix: Cow<'static, str>,
    pub(crate) line_sep: Cow<'static, str>,
    pub(crate) utc_time: bool,
}

#[cfg(feature = "date-based")]
impl DateBased {
    /// Create new date-based file logger with the given file prefix and
    /// strftime-based suffix pattern.
    ///
    /// On initialization, fern will create a file with the suffix formatted
    /// with the current time (either utc or local, see below). Each time a
    /// record is logged, the format is checked against the current time, and if
    /// the time has changed, the old file is closed and a new one opened.
    ///
    /// `file_suffix` will be interpreted as an `strftime` format. See
    /// [`chrono::format::strftime`] for more information.
    ///
    /// `file_prefix` may be a full file path, and will be prepended to the
    /// suffix to create the final file.
    ///
    /// Note that no separator will be placed in between `file_name` and
    /// `file_suffix_pattern`. So if you call `DateBased::new("hello",
    /// "%Y")`, the result will be a filepath `hello2019`.
    ///
    /// By default, this will use local time. For UTC time instead, use the
    /// [`.utc_time()`][DateBased::utc_time] method after creating.
    ///
    /// By default, this will use `\n` as a line separator. For a custom
    /// separator, use the [`.line_sep`][DateBased::line_sep] method
    /// after creating.
    ///
    /// # Examples
    ///
    /// Containing the date (year, month and day):
    ///
    /// ```
    /// // logs/2019-10-23-my-program.log
    /// let log = fern::DateBased::new("logs/", "%Y-%m-%d-my-program.log");
    ///
    /// // program.log.23102019
    /// let log = fern::DateBased::new("my-program.log.", "%d%m%Y");
    /// ```
    ///
    /// Containing the hour:
    ///
    /// ```
    /// // logs/2019-10-23 13 my-program.log
    /// let log = fern::DateBased::new("logs/", "%Y-%m-%d %H my-program.log");
    ///
    /// // program.log.2310201913
    /// let log = fern::DateBased::new("my-program.log.", "%d%m%Y%H");
    /// ```
    ///
    /// Containing the minute:
    ///
    /// ```
    /// // logs/2019-10-23 13 my-program.log
    /// let log = fern::DateBased::new("logs/", "%Y-%m-%d %H my-program.log");
    ///
    /// // program.log.2310201913
    /// let log = fern::DateBased::new("my-program.log.", "%d%m%Y%H");
    /// ```
    ///
    /// UNIX time, or seconds since 00:00 Jan 1st 1970:
    ///
    /// ```
    /// // logs/1571822854-my-program.log
    /// let log = fern::DateBased::new("logs/", "%s-my-program.log");
    ///
    /// // program.log.1571822854
    /// let log = fern::DateBased::new("my-program.log.", "%s");
    /// ```
    ///
    /// Hourly, using UTC time:
    ///
    /// ```
    /// // logs/2019-10-23 23 my-program.log
    /// let log = fern::DateBased::new("logs/", "%Y-%m-%d %H my-program.log").utc_time();
    ///
    /// // program.log.2310201923
    /// let log = fern::DateBased::new("my-program.log.", "%d%m%Y%H").utc_time();
    /// ```
    ///
    /// [`chrono::format::strftime`]: https://docs.rs/chrono/0.4.6/chrono/format/strftime/index.html
    pub fn new<T, U>(file_prefix: T, file_suffix: U) -> Self
    where
        T: AsRef<Path>,
        U: Into<Cow<'static, str>>,
    {
        DateBased {
            utc_time: false,
            file_prefix: file_prefix.as_ref().to_owned(),
            file_suffix: file_suffix.into(),
            line_sep: "\n".into(),
        }
    }

    /// Changes the line separator this logger will use.
    ///
    /// The default line separator is `\n`.
    ///
    /// # Examples
    ///
    /// Using a windows line separator:
    ///
    /// ```
    /// let log = fern::DateBased::new("logs", "%s.log").line_sep("\r\n");
    /// ```
    pub fn line_sep<T>(mut self, line_sep: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.line_sep = line_sep.into();
        self
    }

    /// Orients this log file suffix formatting to use UTC time.
    ///
    /// The default is local time.
    ///
    /// # Examples
    ///
    /// This will use UTC time to determine the date:
    ///
    /// ```
    /// // program.log.2310201923
    /// let log = fern::DateBased::new("my-program.log.", "%d%m%Y%H").utc_time();
    /// ```
    pub fn utc_time(mut self) -> Self {
        self.utc_time = true;
        self
    }

    /// Orients this log file suffix formatting to use local time.
    ///
    /// This is the default option.
    ///
    /// # Examples
    ///
    /// This log file will use local time - the latter method call overrides the
    /// former.
    ///
    /// ```
    /// // program.log.2310201923
    /// let log = fern::DateBased::new("my-program.log.", "%d%m%Y%H")
    ///     .utc_time()
    ///     .local_time();
    /// ```
    pub fn local_time(mut self) -> Self {
        self.utc_time = false;
        self
    }
}


/// File logger with a dynamic time-based name.
#[derive(Debug)]
#[cfg(feature = "date-based")]
pub(crate) struct DateBasedImpl {
    pub(crate) config: DateBasedConfig,
    pub(crate) state: Mutex<DateBasedState>,
}

#[derive(Debug)]
#[cfg(feature = "date-based")]
pub(crate) enum ConfiguredTimezone {
    Local,
    Utc,
}

#[derive(Debug)]
#[cfg(feature = "date-based")]
pub(crate) struct DateBasedConfig {
    pub line_sep: Cow<'static, str>,
    /// This is a Path not an str so it can hold invalid UTF8 paths correctly.
    pub file_prefix: PathBuf,
    pub file_suffix: Cow<'static, str>,
    pub timezone: ConfiguredTimezone,
}

#[derive(Debug)]
#[cfg(feature = "date-based")]
pub(crate) struct DateBasedState {
    pub current_suffix: String,
    pub file_stream: Option<BufWriter<fs::File>>,
}

#[cfg(feature = "date-based")]
impl DateBasedState {
    pub fn new(current_suffix: String, file_stream: Option<fs::File>) -> Self {
        DateBasedState {
            current_suffix,
            file_stream: file_stream.map(BufWriter::new),
        }
    }

    pub fn replace_file(&mut self, new_suffix: String, new_file: Option<fs::File>) {
        if let Some(mut old) = self.file_stream.take() {
            let _ = old.flush();
        }
        self.current_suffix = new_suffix;
        self.file_stream = new_file.map(BufWriter::new)
    }
}

#[cfg(feature = "date-based")]
impl DateBasedConfig {
    pub fn new(
        line_sep: Cow<'static, str>,
        file_prefix: PathBuf,
        file_suffix: Cow<'static, str>,
        timezone: ConfiguredTimezone,
    ) -> Self {
        DateBasedConfig {
            line_sep,
            file_prefix,
            file_suffix,
            timezone,
        }
    }

    pub fn compute_current_suffix(&self) -> String {
        match self.timezone {
            ConfiguredTimezone::Utc => chrono::Utc::now().format(&self.file_suffix).to_string(),
            ConfiguredTimezone::Local => chrono::Local::now().format(&self.file_suffix).to_string(),
        }
    }

    pub fn compute_file_path(&self, suffix: &str) -> PathBuf {
        let mut path = OsString::from(&*self.file_prefix);
        // use the OsString::push method, not PathBuf::push which would add a path
        // separator
        path.push(suffix);
        path.into()
    }

    pub fn open_log_file(path: &Path) -> io::Result<fs::File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)
    }

    pub fn open_current_log_file(&self, suffix: &str) -> io::Result<fs::File> {
        Self::open_log_file(&self.compute_file_path(suffix))
    }
}

#[cfg(feature = "date-based")]
impl Log for DateBasedImpl {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            // Formatting first prevents deadlocks on file-logging,
            // when the process of formatting itself is logged.
            // note: this is only ever needed if some Debug, Display, or other
            // formatting trait itself is logging.
            #[cfg(feature = "meta-logging-in-format")]
            let msg = format!("{}{}", record.args(), self.config.line_sep);

            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

            // check if log needs to be rotated
            let new_suffix = self.config.compute_current_suffix();
            if state.file_stream.is_none() || state.current_suffix != new_suffix {
                let file_open_result = self.config.open_current_log_file(&new_suffix);
                match file_open_result {
                    Ok(file) => {
                        state.replace_file(new_suffix, Some(file));
                    }
                    Err(e) => {
                        state.replace_file(new_suffix, None);
                        return Err(e.into());
                    }
                }
            }

            // either just initialized writer above, or already errored out.
            let writer = state.file_stream.as_mut().unwrap();

            #[cfg(feature = "meta-logging-in-format")]
            write!(writer, "{}", msg)?;
            #[cfg(not(feature = "meta-logging-in-format"))]
            write!(writer, "{}{}", record.args(), self.config.line_sep)?;

            writer.flush()?;

            Ok(())
        });
    }

    fn flush(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(stream) = &mut state.file_stream {
            let _ = stream.flush();
        }
    }
}

#[cfg(feature = "date-based")]
impl From<DateBased> for Output {
    /// Create an output logger which defers to the given date-based logger. Use
    /// configuration methods on [DateBased] to set line separator and filename.
    fn from(config: DateBased) -> Self {
        let config = DateBasedConfig::new(
            config.line_sep,
            config.file_prefix,
            config.file_suffix,
            if config.utc_time {
                ConfiguredTimezone::Utc
            } else {
                ConfiguredTimezone::Local
            },
        );
        let computed_suffix = config.compute_current_suffix();
        // ignore errors - we'll just retry later.
        let initial_file = config.open_current_log_file(&computed_suffix).ok();

        Output(OutputInner::Logger(Box::new(DateBasedImpl {
            config,
            state: Mutex::new(DateBasedState::new(computed_suffix, initial_file)),
        })))
    }
}

#[cfg(all(not(windows), feature = "syslog-4"))]
impl From<syslog4::Error> for LogError {
    fn from(error: syslog4::Error) -> Self {
        LogError::Syslog4(error)
    }
}
