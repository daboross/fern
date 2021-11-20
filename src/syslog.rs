/*!
Example usage of `fern` with the `syslog` crate.

Be sure to depend on `syslog` and the `syslog` feature in `Cargo.toml`:

```toml
[dependencies]
fern = { version = "0.5", features = ["syslog-4"] }]
syslog = "4"
```

To use `syslog`, simply create the log you want, and pass it into `Dispatch::chain`:

```no_run
# use syslog4 as syslog;
# fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
let formatter = syslog::Formatter3164 {
    facility: syslog::Facility::LOG_USER,
    hostname: None,
    process: "hello-world".to_owned(),
    pid: 0,
};

fern::Dispatch::new()
    .chain(syslog::unix(formatter)?)
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

---

## Alternate syslog versions

If you're using syslog=4.0.0 exactly, one line "ok" will be printed to stdout on log configuration.
This is [a bug in syslog](https://github.com/Geal/rust-syslog/issues/39), and there is nothing we
can change in fern to fix that.

One way to avoid this is to use an earlier version of syslog, which `fern` also supports. To do
this, depend on `syslog = 3` instead.

```toml
[dependencies]
fern = { version = "0.5", features = ["syslog-3"] }]
syslog = "3"
```

The setup is very similar, except with less configuration to start the syslog logger:

```rust
# use syslog3 as syslog;
# fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
fern::Dispatch::new()
    .chain(syslog::unix(syslog::Facility::LOG_USER)?)
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

The rest of this document applies to both syslog 3 and syslog 4, but the examples will be using
syslog 4 as it is the latest version.

---

One thing with `syslog` is that you don't generally want to apply any log formatting. The system
logger will handle that for you.

However, you probably will want to format messages you also send to stdout! Fortunately, selective
configuration is easy with fern:

```no_run
# use syslog4 as syslog;
# fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
let syslog_formatter = syslog::Formatter3164 {
    facility: syslog::Facility::LOG_USER,
    hostname: None,
    process: "hello-world".to_owned(),
    pid: 0,
};

// top level config
fern::Dispatch::new()
    .chain(
        // console config
        fern::Dispatch::new()
            .level(log::LevelFilter::Debug)
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{}] {}",
                    record.level(),
                    message,
                ))
            })
            .chain(std::io::stdout())
    )
    .chain(
        // syslog config
        fern::Dispatch::new()
            .level(log::LevelFilter::Info)
            .chain(syslog::unix(syslog_formatter)?)
    )
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

With this, all info and above messages will be sent to the syslog with no formatting, and
the messages sent to the console will still look nice as usual.

---

One last pattern you might want to know: creating a log target which must be explicitly mentioned
in order to work.

```no_run
# use syslog4 as syslog;
# fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
# let formatter = syslog::Formatter3164 {
#     facility: syslog::Facility::LOG_USER,
#     hostname: None,
#     process: "hello-world".to_owned(),
#     pid: 0,
# };
fern::Dispatch::new()
    // by default only accept warning messages from libraries so we don't spam
    .level(log::LevelFilter::Warn)
    // but accept Info and Debug if we explicitly mention syslog
    .level_for("explicit-syslog", log::LevelFilter::Debug)
    .chain(syslog::unix(formatter)?)
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

With this configuration, only warning messages will get through by default. If we do want to
send info or debug messages, we can do so explicitly:

```no_run
# use log::{debug, info, warn};
# fn main() {
debug!("this won't get through");
// especially useful if this is from library you depend on.
info!("neither will this");
warn!("this will!");

info!(target: "explicit-syslog", "this will also show up!");
# }
```
*/

use crate::{*, dispatch::{Output, OutputInner}};

use std::{
    collections::HashMap,
    sync::Mutex,
};

use log::Log;

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

/// Passes all messages to the syslog.
/// Creates an output logger which writes all messages to the given syslog
/// output.
///
/// Log levels are translated trace => debug, debug => debug, info =>
/// informational, warn => warning, and error => error.
///
/// This requires the `"syslog-3"` feature.
#[cfg(all(not(windows), feature = "syslog-3"))]
pub fn syslog3(log: syslog3::Logger) -> impl Log + Into<Output> {
    struct Syslog3 {
        inner: syslog3::Logger,
    }
    
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

    impl From<Syslog3> for Output {
        fn from(log: Syslog3) -> Self {
            Output(OutputInner::Logger(Box::new(log)))
        }
    }

    Syslog3 { inner: log }
}


#[cfg(all(not(windows), feature = "syslog-4"))]
pub(crate) type Syslog4Rfc3164Logger = syslog4::Logger<syslog4::LoggerBackend, String, syslog4::Formatter3164>;

/// Passes all messages to the syslog.
#[cfg(all(not(windows), feature = "syslog-4"))]
pub fn syslog4_3164(log: Syslog4Rfc3164Logger) -> impl Log + Into<Output> {
    pub struct Syslog4Rfc3164 {
        pub(crate) inner: Mutex<Syslog4Rfc3164Logger>,
    }
    
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

    impl From<Syslog4Rfc3164> for Output {
        /// Creates an output logger which writes all messages to the given syslog.
        ///
        /// Log levels are translated trace => debug, debug => debug, info =>
        /// informational, warn => warning, and error => error.
        ///
        /// Note that due to https://github.com/Geal/rust-syslog/issues/41,
        /// logging to this backend requires one allocation per log call.
        ///
        /// This is for RFC 3164 loggers. To use an RFC 5424 logger, use the
        /// [`Output::syslog_5424`] helper method.
        ///
        /// This requires the `"syslog-4"` feature.
        fn from(log: Syslog4Rfc3164) -> Self {
            Output(OutputInner::Logger(Box::new(log)))
        }
    }

    Syslog4Rfc3164 {
        inner: Mutex::new(log),
    }
}

#[cfg(all(not(windows), feature = "syslog-4"))]
pub(crate) type Syslog4Rfc5424Logger = syslog4::Logger<
    syslog4::LoggerBackend,
    (i32, HashMap<String, HashMap<String, String>>, String),
    syslog4::Formatter5424,
>;

/// Passes all messages to the syslog.
/// Returns a logger which logs into an RFC5424 syslog.
///
/// This method takes an additional transform method to turn the log data
/// into RFC5424 data.
///
/// I've honestly got no clue what the expected keys and values are for
/// this kind of logging, so I'm just going to link [the rfc] instead.
///
/// If you're an expert on syslog logging and would like to contribute
/// an example to put here, it would be gladly accepted!
///
/// This requires the `"syslog-4"` feature.
///
/// [the rfc]: https://tools.ietf.org/html/rfc5424
#[cfg(all(not(windows), feature = "syslog-4"))]
pub fn syslog4_5424<F>(logger: Syslog4Rfc5424Logger, transform: F) -> impl Log + Into<Output>
where
    F: Fn(&log::Record) -> (i32, HashMap<String, HashMap<String, String>>, String)
        + Sync
        + Send
        + 'static,
{
    pub struct Syslog4Rfc5424<F> {
        pub(crate) inner: Mutex<Syslog4Rfc5424Logger>,
        pub(crate) transform: F,
    }

    impl <F> Log for Syslog4Rfc5424<F> 
    where 
        F: Fn(&log::Record) -> (i32, HashMap<String, HashMap<String, String>>, String)
        + Sync
        + Send
        + 'static
    {
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

    impl <F> From<Syslog4Rfc5424<F> > for Output where 
    F: Fn(&log::Record) -> (i32, HashMap<String, HashMap<String, String>>, String)
    + Sync
    + Send
    + 'static
    {
        fn from(log: Syslog4Rfc5424<F>) -> Self {
            Output(OutputInner::Logger(Box::new(log)))
        }
    }

    Syslog4Rfc5424 {
        inner: Mutex::new(logger),
        transform,
    }
}


// #[cfg(all(not(windows), feature = "syslog-3"))]
// impl From<syslog3::Logger> for Output {
//     /// Creates an output logger which writes all messages to the given syslog
//     /// output.
//     ///
//     /// Log levels are translated trace => debug, debug => debug, info =>
//     /// informational, warn => warning, and error => error.
//     ///
//     /// Note that while this takes a Box<Logger> for convenience (syslog
//     /// methods return Boxes), it will be immediately unboxed upon storage
//     /// in the configuration structure. This will create a configuration
//     /// identical to that created by passing a raw `syslog::Logger`.
//     ///
//     /// This requires the `"syslog-3"` feature.
//     fn from(log: syslog3::Logger) -> Self {
//         Output(OutputInner::Logger(Box::new(log)))
//     }
// }

// #[cfg(all(not(windows), feature = "syslog-3"))]
// impl From<Box<syslog3::Logger>> for Output {
//     /// Creates an output logger which writes all messages to the given syslog
//     /// output.
//     ///
//     /// Log levels are translated trace => debug, debug => debug, info =>
//     /// informational, warn => warning, and error => error.
//     ///
//     /// Note that while this takes a Box<Logger> for convenience (syslog
//     /// methods return Boxes), it will be immediately unboxed upon storage
//     /// in the configuration structure. This will create a configuration
//     /// identical to that created by passing a raw `syslog::Logger`.
//     ///
//     /// This requires the `"syslog-3"` feature.
//     fn from(log: Box<syslog3::Logger>) -> Self {
//         Output(OutputInner::Logger(1log as _))
//     }
// }
