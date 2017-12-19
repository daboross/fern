//! Module containing color-related functions using the `colored` crate.
use colored::{Color, ColoredString, Colorize};
use log::LogLevel;

/// Extension crate allowing the use of `.colored` on LogLevels.
pub trait ColoredLogLevel {
    /// Colors this log level with the given color.
    fn colored(&self, color: Color) -> ColoredString;
}

fn new_colored_string(input: String, color: Color) -> ColoredString {
    let cs = ColoredString::from(input.as_str());
    cs.color(color)
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
    /// Creates a new ColoredLogLevelConfig with the given colors for each log level.
    ///
    /// Use [`ColoredLogLevelConfig::default`] for default colors.
    ///
    /// [`ColoredLogLevelConfig::default`]: #method.default
    #[inline]
    pub fn new(error: Color, warn: Color, info: Color, debug: Color, trace: Color) -> Self {
        ColoredLogLevelConfig {
            trace: trace,
            error: error,
            warn: warn,
            debug: debug,
            info: info,
        }
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
    pub fn color(&self, level: LogLevel) -> ColoredString {
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
    fn colored(&self, color: Color) -> ColoredString {
        let s = format!("{:?}", self);

        new_colored_string(s, color)
    }
}
