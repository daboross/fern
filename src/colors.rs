//! Module containing color-related functions using the `colored` crate.
//!
//! This module is only available when the `colored` feature is enabled for `fern`.
//! This can be done with the following in `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! # ...
//! fern = { version = "0.4", features = ["colored"] }
//! ```
use std::fmt;
use colored::Color;
use log::LogLevel;

/// Extension crate allowing the use of `.colored` on LogLevels.
trait ColoredLogLevel {
    /// Colors this log level with the given color.
    fn colored(&self, color: Color) -> LogLevelWithColor;
}

/// Opaque structure representing a log level with an associated color. This implements [`fmt::Display`] so that it is
/// displayed as the underlying log level surrounded with ASCII markers for the given color.
// this is necessary in order to avoid using colored::ColorString, which has a Display
// implementation involving many allocations, and would involve two more string allocations
// even to create it.
pub struct LogLevelWithColor {
    level: LogLevel,
    color: Color,
}

impl fmt::Display for LogLevelWithColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}m{}\x1B[0m", self.color.to_fg_str(), self.level)
    }
}

/// Configuration specifying colors a log level can be colored as.
#[derive(Copy, Clone)]
pub struct ColoredLogLevelConfig {
    /// The color to color logs with the [`Error`] level.
    ///
    /// [`Error`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Error
    pub error: Color,
    /// The color to color logs with the [`Warn`] level.
    ///
    /// [`Warn`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Warn
    pub warn: Color,
    /// The color to color logs with the [`Info`] level.
    ///
    /// [`Info`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Info
    pub info: Color,
    /// The color to color logs with the [`Debug`] level.
    ///
    /// [`Debug`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Debug
    pub debug: Color,
    /// The color to color logs with the [`Trace`] level.
    ///
    /// [`Trace`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Trace
    pub trace: Color,
}

impl ColoredLogLevelConfig {
    /// Creates a new ColoredLogLevelConfig with the default colors.
    ///
    /// This matches the behavior of [`ColoredLogLevelConfig::default`].
    ///
    /// [`ColoredLogLevelConfig::default`]: #method.default
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the [`Error`] level color with the given color.
    ///
    /// The default color is [`Color::Red`].
    ///
    /// [`Error`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Error
    /// [`Color::Red`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Red
    pub fn error(&mut self, error: Color) -> &mut Self {
        self.error = error;
        self
    }

    /// Overrides the [`Warn`] level color with the given color.
    ///
    /// The default color is [`Color::Yellow`].
    ///
    /// [`Warn`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Warn
    /// [`Color::Yellow`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Yellow
    pub fn warn(&mut self, warn: Color) -> &mut Self {
        self.warn = warn;
        self
    }

    /// Overrides the [`Info`] level color with the given color.
    ///
    /// The default color is [`Color::White`].
    ///
    /// [`Info`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Info
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    pub fn info(&mut self, info: Color) -> &mut Self {
        self.info = info;
        self
    }

    /// Overrides the [`Debug`] level color with the given color.
    ///
    /// The default color is [`Color::White`].
    ///
    /// [`Debug`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Debug
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    pub fn debug(&mut self, debug: Color) -> &mut Self {
        self.debug = debug;
        self
    }

    /// Overrides the [`Trace`] level color with the given color.
    ///
    /// The default color is [`Color::White`].
    ///
    /// [`Trace`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Trace
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    pub fn trace(&mut self, trace: Color) -> &mut Self {
        self.trace = trace;
        self
    }

    /// Retrieves the default configuration. This has:
    ///
    /// - [`Error`] as [`Color::Red`]
    /// - [`Warn`] as [`Color::Yellow`]
    /// - [`Info`] as [`Color::White`]
    /// - [`Debug`] as [`Color::White`]
    /// - [`Trace`] as [`Color::White`]
    ///
    /// [`Error`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Error
    /// [`Warn`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Warn
    /// [`Info`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Info
    /// [`Debug`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Debug
    /// [`Trace`]: https://docs.rs/log/0.3/log/enum.LogLevel.html#variant.Trace
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    /// [`Color::Yellow`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Yellow
    /// [`Color::Red`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Red
    pub fn default() -> Self {
        ColoredLogLevelConfig {
            error: Color::Red,
            warn: Color::Yellow,
            debug: Color::White,
            info: Color::White,
            trace: Color::White,
        }
    }

    /// Colors the given log level with the correct color.
    ///
    /// This will output ANSI escapes correctly coloring the log level when printed
    /// to a Unix terminal. Due to behavior of the [`colored`] crate, this does not
    /// function on Windows terminals.
    ///
    /// [`colored`]: https://github.com/mackwic/colored
    pub fn color(&self, level: LogLevel) -> LogLevelWithColor {
        level.colored(self.get_color(&level))
    }

    /// Retrieves the color that a log level should be colored as.
    pub fn get_color(&self, level: &LogLevel) -> Color {
        match *level {
            LogLevel::Error => self.error,
            LogLevel::Warn => self.warn,
            LogLevel::Info => self.info,
            LogLevel::Debug => self.debug,
            LogLevel::Trace => self.trace,
        }
    }
}

impl ColoredLogLevel for LogLevel {
    fn colored(&self, color: Color) -> LogLevelWithColor {
        LogLevelWithColor {
            level: *self,
            color: color,
        }
    }
}
