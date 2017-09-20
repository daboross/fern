use std::io::{self, BufWriter, Write};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::{fmt, fs};

use std::collections::HashMap;

use log::{self, Log};

use {FernLog, Filter, Formatter};

pub enum LevelConfiguration {
    JustDefault,
    Minimal(Vec<(Cow<'static, str>, log::LogLevelFilter)>),
    Many(HashMap<Cow<'static, str>, log::LogLevelFilter>),
}

pub struct Dispatch {
    pub output: Vec<Output>,
    pub default_level: log::LogLevelFilter,
    pub levels: LevelConfiguration,
    pub format: Option<Box<Formatter>>,
    pub filters: Vec<Box<Filter>>,
}

/// Callback struct for use within a formatter closure
///
/// Callbacks are used for formatting in order to allow usage of [`std::fmt`]-based formatting without
/// the allocation of the formatted result which would be required to return it.
///
/// Example usage:
///
/// ```
/// fern::Dispatch::new()
///     .format(|callback: fern::FormatCallback, message, record| {
///         callback.finish(format_args!("[{}] {}", record.level(), message))
///     })
///     # ;
/// ```
///
/// [`std::fmt`]: https://doc.rust-lang.org/std/fmt/index.html
#[must_use = "format callback must be used for log to process correctly"]
pub struct FormatCallback<'a>(InnerFormatCallback<'a>);

struct InnerFormatCallback<'a>(&'a mut bool, &'a Dispatch, &'a log::LogRecord<'a>);

pub enum Output {
    Stdout(Stdout),
    Stderr(Stderr),
    File(File),
    Sender(Sender),
    Dispatch(Dispatch),
    SharedDispatch(Arc<Dispatch>),
    Other(Box<FernLog>),
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

pub struct Null;

impl From<Vec<(Cow<'static, str>, log::LogLevelFilter)>> for LevelConfiguration {
    fn from(mut levels: Vec<(Cow<'static, str>, log::LogLevelFilter)>) -> Self {
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
    fn find_module(&self, module: &str) -> Option<log::LogLevelFilter> {
        match *self {
            LevelConfiguration::JustDefault => None,
            _ => {
                if let Some(level) = self.find_exact(module) {
                    return Some(level);
                }

                // The manual for loop here lets us just iterate over the module string once while
                // still finding each sub-module. For the module string "hyper::http::h1", this loop
                // will test first "hyper::http" then "hyper".
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

    fn find_exact(&self, module: &str) -> Option<log::LogLevelFilter> {
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

impl FernLog for Output {
    fn log_args(&self, input: &fmt::Arguments, record: &log::LogRecord) {
        match *self {
            Output::Stdout(ref s) => s.log_args(input, record),
            Output::Stderr(ref s) => s.log_args(input, record),
            Output::File(ref s) => s.log_args(input, record),
            Output::Sender(ref s) => s.log_args(input, record),
            Output::Dispatch(ref s) => s.log_args(input, record),
            Output::SharedDispatch(ref s) => s.log_args(input, record),
            Output::Other(ref s) => s.log_args(input, record),
        }
    }
}

impl log::Log for Dispatch {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        metadata.level()
            <= self.levels
                .find_module(metadata.target())
                .unwrap_or(self.default_level) && self.filters.iter().all(|f| f(metadata))
    }

    fn log(&self, record: &log::LogRecord) {
        self.log_args(record.args(), record)
    }
}

impl log::Log for Null {
    fn enabled(&self, _: &log::LogMetadata) -> bool {
        false
    }

    fn log(&self, _: &log::LogRecord) {}
}

impl FernLog for Dispatch {
    fn log_args(&self, message: &fmt::Arguments, record: &log::LogRecord) {
        if self.enabled(record.metadata()) {
            match self.format {
                Some(ref format) => {
                    // flag to ensure the log message is completed even if the formatter doesn't complete the callback.
                    let mut callback_called_flag = false;

                    (format)(
                        FormatCallback(InnerFormatCallback(&mut callback_called_flag, self, record)),
                        message,
                        record,
                    );

                    if !callback_called_flag {
                        self.finish_logging(message, record);
                    }
                }
                None => {
                    self.finish_logging(message, record);
                }
            }
        }
    }
}

impl Dispatch {
    fn finish_logging(&self, formatted_message: &fmt::Arguments, record: &log::LogRecord) {
        for log in &self.output {
            log.log_args(formatted_message, record);
        }
    }
}

impl<'a> FormatCallback<'a> {
    /// Complete the formatting call that this FormatCallback was created for.
    ///
    /// This will call the rest of the logging chain using the given formatted message
    /// as the new payload message.
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

        dispatch.finish_logging(&formatted_message, record);
    }
}

// No need to write this twice (used for Stdout and Stderr structs)
macro_rules! std_log_impl {
    ($ident:ident) => {
        impl FernLog for $ident {
            fn log_args(&self, payload: &fmt::Arguments, record: &log::LogRecord) {
                fallback_on_error(payload, record, |payload, _| {
                    write!(self.stream.lock(), "{}{}", payload, self.line_sep)?;
                    Ok(())
                });
            }
        }
    };
}

std_log_impl!(Stdout);
std_log_impl!(Stderr);

impl FernLog for File {
    fn log_args(&self, payload: &fmt::Arguments, record: &log::LogRecord) {
        fallback_on_error(payload, record, |payload, _| {
            let mut writer = self.stream.lock().unwrap_or_else(|e| e.into_inner());

            write!(writer, "{}{}", payload, self.line_sep)?;

            writer.flush()?;

            Ok(())
        });
    }
}

impl FernLog for Sender {
    fn log_args(&self, payload: &fmt::Arguments, record: &log::LogRecord) {
        fallback_on_error(payload, record, |payload, _| {
            let msg = format!("{}{}", payload, self.line_sep);
            self.stream
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .send(msg)?;
            Ok(())
        });
    }
}

#[inline(always)]
fn fallback_on_error<F>(payload: &fmt::Arguments, record: &log::LogRecord, log_func: F)
where
    F: FnOnce(&fmt::Arguments, &log::LogRecord) -> Result<(), LogError>,
{
    if let Err(error) = log_func(payload, record) {
        backup_logging(payload, record, &error)
    }
}

fn backup_logging(payload: &fmt::Arguments, record: &log::LogRecord, error: &LogError) {
    let second = write!(
        io::stderr(),
        "Error performing logging.\
         \n\tattempted to log: {}\
         \n\torigin location: {:#?}\
         \n\tlogging error: {}",
        payload,
        record.location(),
        error
    );

    if let Err(second_error) = second {
        panic!(
            "Error performing stderr logging after error occurred during regular logging.\
             \n\tattempted to log: {}\
             \n\torigin location: {:#?}\
             \n\tfirst logging Error: {}\
             \n\tstderr error: {}",
            payload,
            record.location(),
            error,
            second_error
        );
    }
}

#[derive(Debug)]
enum LogError {
    Io(io::Error),
    Send(mpsc::SendError<String>),
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogError::Io(ref e) => write!(f, "{}", e),
            LogError::Send(ref e) => write!(f, "{}", e),
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

#[cfg(test)]
mod test {
    use super::LevelConfiguration;
    use log::LogLevelFilter::*;

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
            ].into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        );


        assert_eq!(config.find_module("root"), Some(Trace));
        assert_eq!(config.find_module("root::other_module"), Some(Trace));

        // We want to ensure that it does pick up most specific level before trying anything more general.
        assert_eq!(config.find_module("root::sub1"), Some(Debug));
        assert_eq!(config.find_module("root::sub1::other_module"), Some(Debug));

        assert_eq!(config.find_module("root::sub2"), Some(Info));
        assert_eq!(config.find_module("root::sub2::other"), Some(Info));

        assert_eq!(config.find_module("root::sub2::sub2.3"), Some(Warn));
        assert_eq!(config.find_module("root::sub2::sub2.3::sub2.4"), Some(Error));

        assert_eq!(config.find_module("root::sub3"), Some(Off));
        assert_eq!(config.find_module("root::sub3::any::children::of::sub3"), Some(Off));
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
            vec![("root", Trace), ("root::su", Debug), ("root::su:b2", Info), ("root::sub2", Warn)]
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
