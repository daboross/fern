use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    fmt, fs,
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    sync::{mpsc, Arc, Mutex},
};

use log::{self, Log};

use crate::{Filter, Formatter};

#[cfg(all(not(windows), feature = "syslog-4"))]
use crate::{Syslog4Rfc3164Logger, Syslog4Rfc5424Logger};
#[cfg(all(not(windows), feature = "reopen-03"))]
use reopen;

pub enum LevelConfiguration {
    JustDefault,
    Minimal(Vec<(Cow<'static, str>, log::LevelFilter)>),
    Many(HashMap<Cow<'static, str>, log::LevelFilter>),
}

pub struct Dispatch {
    pub output: Vec<Output>,
    pub default_level: log::LevelFilter,
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

struct InnerFormatCallback<'a>(&'a mut bool, &'a Dispatch, &'a log::Record<'a>);

pub enum Output {
    Stdout(Stdout),
    Stderr(Stderr),
    File(File),
    Sender(Sender),
    #[cfg(all(not(windows), feature = "syslog-3"))]
    Syslog3(Syslog3),
    #[cfg(all(not(windows), feature = "syslog-4"))]
    Syslog4Rfc3164(Syslog4Rfc3164),
    #[cfg(all(not(windows), feature = "syslog-4"))]
    Syslog4Rfc5424(Syslog4Rfc5424),
    Dispatch(Dispatch),
    SharedDispatch(Arc<Dispatch>),
    OtherBoxed(Box<dyn Log>),
    OtherStatic(&'static dyn Log),
    Panic(Panic),
    Writer(Writer),
    DateBasedLogFile(DateBasedLogFile),
    #[cfg(all(not(windows), feature = "reopen-03"))]
    Reopen(Reopen),
}

pub struct Stdout {
    pub stream: io::Stdout,
    pub line_sep: Cow<'static, str>,
}

pub struct Stderr {
    pub stream: io::Stderr,
    pub line_sep: Cow<'static, str>,
}

pub struct File {
    pub stream: Mutex<BufWriter<fs::File>>,
    pub line_sep: Cow<'static, str>,
}

pub struct Sender {
    pub stream: Mutex<mpsc::Sender<String>>,
    pub line_sep: Cow<'static, str>,
}

pub struct Writer {
    pub stream: Mutex<Box<dyn Write + Send>>,
    pub line_sep: Cow<'static, str>,
}

#[cfg(all(not(windows), feature = "reopen-03"))]
pub struct Reopen {
    pub stream: Mutex<reopen::Reopen<fs::File>>,
    pub line_sep: Cow<'static, str>,
}

#[cfg(all(not(windows), feature = "syslog-3"))]
pub struct Syslog3 {
    pub inner: syslog3::Logger,
}

#[cfg(all(not(windows), feature = "syslog-4"))]
pub struct Syslog4Rfc3164 {
    pub inner: Mutex<Syslog4Rfc3164Logger>,
}

#[cfg(all(not(windows), feature = "syslog-4"))]
pub struct Syslog4Rfc5424 {
    pub inner: Mutex<Syslog4Rfc5424Logger>,
    pub transform: Box<
        dyn Fn(&log::Record) -> (i32, HashMap<String, HashMap<String, String>>, String)
            + Sync
            + Send,
    >,
}

pub struct Panic;

pub struct Null;

/// File logger with a dynamic time-based name.
#[derive(Debug)]
pub struct DateBasedLogFile {
    pub line_sep: Cow<'static, str>,
    pub file_suffix: Cow<'static, str>,
    pub file_name: Cow<'static, str>,
    pub log_file_state: Mutex<DateBasedLogFileState>,
}

#[derive(Debug)]
pub struct DateBasedLogFileState {
    curr_file_suffix: RefCell<String>,
    is_writeable: RefCell<bool>,
    file_stream: RefCell<BufWriter<fs::File>>,
}

impl DateBasedLogFileState {
    pub fn new(
        curr_file_suffix: &str,
        is_writeable: bool,
        file_stream: BufWriter<fs::File>,
    ) -> DateBasedLogFileState {
        DateBasedLogFileState {
            curr_file_suffix: RefCell::new(curr_file_suffix.to_string()),
            is_writeable: RefCell::new(is_writeable),
            file_stream: RefCell::new(file_stream),
        }
    }
}

impl DateBasedLogFile {
    pub fn get_suffix(file_suffix_pattern: &str) -> String {
        chrono::Local::now().format(file_suffix_pattern).to_string()
    }

    pub fn get_file_name(file_name: &str, file_suffix: &str) -> String {
        let mut file_name_full = file_name.to_string();
        file_name_full.push('.');
        file_name_full.push_str(&file_suffix);
        file_name_full
    }

    pub fn open_log_file(path: &str) -> io::Result<fs::File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)
    }
}

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

impl Log for Output {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        match *self {
            Output::Stdout(ref s) => s.enabled(metadata),
            Output::Stderr(ref s) => s.enabled(metadata),
            Output::File(ref s) => s.enabled(metadata),
            Output::Sender(ref s) => s.enabled(metadata),
            Output::Dispatch(ref s) => s.enabled(metadata),
            Output::SharedDispatch(ref s) => s.enabled(metadata),
            Output::OtherBoxed(ref s) => s.enabled(metadata),
            Output::OtherStatic(ref s) => s.enabled(metadata),
            #[cfg(all(not(windows), feature = "syslog-3"))]
            Output::Syslog3(ref s) => s.enabled(metadata),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            Output::Syslog4Rfc3164(ref s) => s.enabled(metadata),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            Output::Syslog4Rfc5424(ref s) => s.enabled(metadata),
            Output::Panic(ref s) => s.enabled(metadata),
            Output::Writer(ref s) => s.enabled(metadata),
            Output::DateBasedLogFile(ref s) => s.enabled(metadata),
            #[cfg(all(not(windows), feature = "reopen-03"))]
            Output::Reopen(ref s) => s.enabled(metadata),
        }
    }

    fn log(&self, record: &log::Record) {
        match *self {
            Output::Stdout(ref s) => s.log(record),
            Output::Stderr(ref s) => s.log(record),
            Output::File(ref s) => s.log(record),
            Output::Sender(ref s) => s.log(record),
            Output::Dispatch(ref s) => s.log(record),
            Output::SharedDispatch(ref s) => s.log(record),
            Output::OtherBoxed(ref s) => s.log(record),
            Output::OtherStatic(ref s) => s.log(record),
            #[cfg(all(not(windows), feature = "syslog-3"))]
            Output::Syslog3(ref s) => s.log(record),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            Output::Syslog4Rfc3164(ref s) => s.log(record),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            Output::Syslog4Rfc5424(ref s) => s.log(record),
            Output::Panic(ref s) => s.log(record),
            Output::Writer(ref s) => s.log(record),
            Output::DateBasedLogFile(ref s) => s.log(record),
            #[cfg(all(not(windows), feature = "reopen-03"))]
            Output::Reopen(ref s) => s.log(record),
        }
    }

    fn flush(&self) {
        match *self {
            Output::Stdout(ref s) => s.flush(),
            Output::Stderr(ref s) => s.flush(),
            Output::File(ref s) => s.flush(),
            Output::Sender(ref s) => s.flush(),
            Output::Dispatch(ref s) => s.flush(),
            Output::SharedDispatch(ref s) => s.flush(),
            Output::OtherBoxed(ref s) => s.flush(),
            Output::OtherStatic(ref s) => s.flush(),
            #[cfg(all(not(windows), feature = "syslog-3"))]
            Output::Syslog3(ref s) => s.flush(),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            Output::Syslog4Rfc3164(ref s) => s.flush(),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            Output::Syslog4Rfc5424(ref s) => s.flush(),
            Output::Panic(ref s) => s.flush(),
            Output::Writer(ref s) => s.flush(),
            Output::DateBasedLogFile(ref s) => s.flush(),
            #[cfg(all(not(windows), feature = "reopen-03"))]
            Output::Reopen(ref s) => s.flush(),
        }
    }
}

impl Log for Null {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _: &log::Record) {}

    fn flush(&self) {}
}

impl Log for Dispatch {
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

impl Dispatch {
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

// No need to write this twice (used for Stdout and Stderr structs)
macro_rules! std_log_impl {
    ($ident:ident) => {
        impl Log for $ident {
            fn enabled(&self, _: &log::Metadata) -> bool {
                true
            }

            fn log(&self, record: &log::Record) {
                fallback_on_error(record, |record| {
                    if cfg!(feature = "meta-logging-in-format") {
                        // Formatting first prevents deadlocks when the process of formatting
                        // itself is logged. note: this is only ever needed if some
                        // Debug, Display, or other formatting trait itself is
                        // logging things too.
                        let msg = format!("{}{}", record.args(), self.line_sep);

                        write!(self.stream.lock(), "{}", msg)?;
                    } else {
                        write!(self.stream.lock(), "{}{}", record.args(), self.line_sep)?;
                    }

                    Ok(())
                });
            }

            fn flush(&self) {
                let _ = self.stream.lock().flush();
            }
        }
    };
}

std_log_impl!(Stdout);
std_log_impl!(Stderr);

macro_rules! writer_log_impl {
    ($ident:ident) => {
        impl Log for $ident {
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
                let _ = self
                    .stream
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .flush();
            }
        }
    };
}

writer_log_impl!(File);
writer_log_impl!(Writer);

#[cfg(all(not(windows), feature = "reopen-03"))]
writer_log_impl!(Reopen);

impl Log for Sender {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            let msg = format!("{}{}", record.args(), self.line_sep);
            self.stream
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .send(msg)?;
            Ok(())
        });
    }

    fn flush(&self) {}
}

#[cfg(any(feature = "syslog-3", feature = "syslog-4"))]
macro_rules! send_syslog {
    ($logger:expr, $level:expr, $message:expr) => {
        use log::Level;
        match $level {
            Level::Error => $logger.err($message)?,
            Level::Warn => $logger.warning($message)?,
            Level::Info => $logger.info($message)?,
            Level::Debug | Level::Trace => $logger.debug($message)?,
        }
    };
}

#[cfg(all(not(windows), feature = "syslog-3"))]
impl Log for Syslog3 {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            let message = record.args();
            send_syslog!(self.inner, record.level(), message);

            Ok(())
        });
    }
    fn flush(&self) {}
}

#[cfg(all(not(windows), feature = "syslog-4"))]
impl Log for Syslog4Rfc3164 {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            let message = record.args().to_string();
            let mut log = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            send_syslog!(log, record.level(), message);

            Ok(())
        });
    }
    fn flush(&self) {}
}

#[cfg(all(not(windows), feature = "syslog-4"))]
impl Log for Syslog4Rfc5424 {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            let transformed = (self.transform)(record);
            let mut log = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            send_syslog!(log, record.level(), transformed);

            Ok(())
        });
    }
    fn flush(&self) {}
}

impl Log for Panic {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        panic!("{}", record.args());
    }

    fn flush(&self) {}
}

impl Log for DateBasedLogFile {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        fallback_on_error(record, |record| {
            let log_file_state = self
                .log_file_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());

            if *log_file_state.is_writeable.borrow() {
                if cfg!(feature = "meta-logging-in-format") {
                    // Formatting first prevents deadlocks on file-logging,
                    // when the process of formatting itself is logged.
                    // note: this is only ever needed if some Debug, Display, or other
                    // formatting trait itself is logging.
                    let msg = format!("{}{}", record.args(), self.line_sep);

                    let mut writer = log_file_state.file_stream.borrow_mut();

                    write!(writer, "{}", msg)?;

                    writer.flush()?;
                } else {
                    let mut writer = log_file_state.file_stream.borrow_mut();

                    write!(writer, "{}{}", record.args(), self.line_sep)?;

                    writer.flush()?;
                }

                //now check if log needs to be rotated
                let new_file_suffix = DateBasedLogFile::get_suffix(&self.file_suffix);
                let mut curr_file_suffix = log_file_state.curr_file_suffix.borrow_mut();

                if !curr_file_suffix.eq(&new_file_suffix) {
                    *curr_file_suffix = new_file_suffix;

                    let file_name_full =
                        DateBasedLogFile::get_file_name(&self.file_name, &curr_file_suffix);
                    let file_open_result = DateBasedLogFile::open_log_file(&file_name_full);

                    if let Ok(l_file) = file_open_result {
                        let mut file_stream = log_file_state.file_stream.borrow_mut();
                        *file_stream = BufWriter::new(l_file);
                    } else {
                        let mut is_writeable = log_file_state.is_writeable.borrow_mut();
                        *is_writeable = false;
                    }
                }

                Ok(())
            } else {
                Err(LogError::CannotOpenFile)
            }
        });
    }

    fn flush(&self) {
        let log_file_state = self
            .log_file_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if *log_file_state.is_writeable.borrow() {
            let _ = log_file_state.file_stream.borrow_mut().flush();
        }
    }
}

#[inline(always)]
fn fallback_on_error<F>(record: &log::Record, log_func: F)
where
    F: FnOnce(&log::Record) -> Result<(), LogError>,
{
    if let Err(error) = log_func(record) {
        backup_logging(record, &error)
    }
}

fn backup_logging(record: &log::Record, error: &LogError) {
    let second = write!(
        io::stderr(),
        "Error performing logging.\
         \n\tattempted to log: {}\
         \n\trecord: {:?}\
         \n\tlogging error: {}",
        record.args(),
        record,
        error
    );

    if let Err(second_error) = second {
        panic!(
            "Error performing stderr logging after error occurred during regular logging.\
             \n\tattempted to log: {}\
             \n\trecord: {:?}\
             \n\tfirst logging error: {}\
             \n\tstderr error: {}",
            record.args(),
            record,
            error,
            second_error,
        );
    }
}

#[derive(Debug)]
enum LogError {
    Io(io::Error),
    Send(mpsc::SendError<String>),
    #[cfg(all(not(windows), feature = "syslog-4"))]
    Syslog4(syslog4::Error),
    CannotOpenFile,
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogError::Io(ref e) => write!(f, "{}", e),
            LogError::Send(ref e) => write!(f, "{}", e),
            #[cfg(all(not(windows), feature = "syslog-4"))]
            LogError::Syslog4(ref e) => write!(f, "{}", e),
            LogError::CannotOpenFile => write!(f, "cannot open log file for writing."),
        }
    }
}

impl From<io::Error> for LogError {
    fn from(error: io::Error) -> Self {
        LogError::Io(error)
    }
}

impl From<mpsc::SendError<String>> for LogError {
    fn from(error: mpsc::SendError<String>) -> Self {
        LogError::Send(error)
    }
}

#[cfg(all(not(windows), feature = "syslog-4"))]
impl From<syslog4::Error> for LogError {
    fn from(error: syslog4::Error) -> Self {
        LogError::Syslog4(error)
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
