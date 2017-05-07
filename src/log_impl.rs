use std::io::{self, Write, BufWriter};
use std::borrow::Cow;
use std::sync::Mutex;
use std::fs;
use std::fmt;

use std::collections::HashMap;

use log::{self, Log};

use {FernLog, Formatter, Filter};

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

/// A format callback, for use within a formatter closure
///
/// Callbacks are used for formatting in order to allow usage of rust's macro-based formatting efficiently
/// and without any extra allocations for intermediate string results.
///
/// Example usage:
///
/// ```
/// fern::Dispatch::new()
///     .format(|callback: fern::FormatCallback, message, record| {
///         callback.finish(format_args!("[{}] {}", record.level(), message));
///     })
///     # ;
/// ```
#[must_use = "format callback must be used for log to process correctly"]
pub struct FormatCallback<'a>(&'a mut bool, &'a Dispatch, &'a log::LogRecord<'a>);

pub enum Output {
    Stdout(Stdout),
    Stderr(Stderr),
    File(File),
    Dispatch(Dispatch),
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
    #[inline]
    fn find(&self, module: &str) -> Option<log::LogLevelFilter> {
        match *self {
            LevelConfiguration::JustDefault => None,
            LevelConfiguration::Minimal(ref levels) => {
                levels.iter().find(|&&(ref test_module, _)| test_module == module).map(|&(_, level)| level)
            }
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
            Output::Dispatch(ref s) => s.log_args(input, record),
            Output::Other(ref s) => s.log_args(input, record),
        }
    }
}

impl log::Log for Dispatch {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        metadata.level() <= self.levels.find(metadata.target()).unwrap_or(self.default_level) &&
        self.filters.iter().all(|f| f(metadata))
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

                    (format)(FormatCallback(&mut callback_called_flag, self, record),
                             message,
                             record);

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
    /// Note: if this is not called, the log message will still be processed, but the
    /// original message will be used, not the reformatted version.
    pub fn finish(self, formatted_message: fmt::Arguments) {
        let FormatCallback(callback_called_flag, dispatch, record) = self;

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
                    write!(self.stream.lock(), "{}{}", payload, self.line_sep)
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

#[inline(always)]
fn fallback_on_error<F>(payload: &fmt::Arguments, record: &log::LogRecord, log_func: F)
    where F: FnOnce(&fmt::Arguments, &log::LogRecord) -> io::Result<()>
{
    if let Err(error) = log_func(payload, record) {
        backup_logging(payload, record, error)
    }
}

fn backup_logging(payload: &fmt::Arguments, record: &log::LogRecord, error: io::Error) {
    let second = write!(io::stderr(),
                        "Error performing logging.\
                            \n\tattempted to log: {}\
                            \n\torigin location: {:#?}\
                            \n\tlogging error: {}",
                        payload,
                        record.location(),
                        error);

    if let Err(second_error) = second {
        panic!("Error performing stderr logging after error occurred during regular logging.\
                \n\tattempted to log: {}\
                \n\torigin location: {:#?}\
                \n\tfirst logging Error: {}\
                \n\tstderr error: {}",
               payload,
               record.location(),
               error,
               second_error);
    }
}
