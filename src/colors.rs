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
pub use colored::Color;
use log::Level;

/// Extension crate allowing the use of `.colored` on Levels.
trait ColoredLogLevel {
    /// Colors this log level with the given color.
    fn colored(&self, color: Color) -> WithFgColor<Level>;
}

/// Opaque structure which represents some text data and a color to display it with. This implements [`fmt::Display`]
/// via displaying the inner text (usually a log level) with ANSI color markers before to set the color and after to
/// reset the color.
///
/// WithFgColor instances can be created and displayed without any allocation.
// this is necessary in order to avoid using colored::ColorString, which has a Display
// implementation involving many allocations, and would involve two more string allocations
// even to create it.
pub struct WithFgColor<T>
where
    T: fmt::Display,
{
    text: T,
    color: Color,
}

impl<T> fmt::Display for WithFgColor<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}m{}\x1B[0m", self.color.to_fg_str(), self.text)
    }
}

/// Configuration specifying colors a log level can be colored as.
///
/// Example usage setting custom 'info' and 'debug' colors:
///
/// ```
/// use fern::colors::{Color, ColoredLevelConfig};
///
/// let colors = ColoredLevelConfig::new()
///     .info(Color::Green)
///     .debug(Color::Magenta);
///
/// fern::Dispatch::new()
///     .format(move |out, message, record| {
///         out.finish(format_args!(
///             "[{}] {}",
///             colors.color(record.level()),
///             message
///         ))
///     })
///     .chain(std::io::stdout())
/// # /*
///     .apply()?;
/// # */
/// #   .into_log();
/// ```
#[derive(Copy, Clone)]
#[must_use = "builder methods take config by value and thus must be reassigned to variable"]
pub struct ColoredLevelConfig {
    /// The color to color logs with the [`Error`] level.
    ///
    /// [`Error`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Error
    pub error: Color,
    /// The color to color logs with the [`Warn`] level.
    ///
    /// [`Warn`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Warn
    pub warn: Color,
    /// The color to color logs with the [`Info`] level.
    ///
    /// [`Info`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Info
    pub info: Color,
    /// The color to color logs with the [`Debug`] level.
    ///
    /// [`Debug`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Debug
    pub debug: Color,
    /// The color to color logs with the [`Trace`] level.
    ///
    /// [`Trace`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Trace
    pub trace: Color,
}

impl ColoredLevelConfig {
    /// Creates a new ColoredLevelConfig with the default colors.
    ///
    /// This matches the behavior of [`ColoredLevelConfig::default`].
    ///
    /// [`ColoredLevelConfig::default`]: #method.default
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the [`Error`] level color with the given color.
    ///
    /// The default color is [`Color::Red`].
    ///
    /// [`Error`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Error
    /// [`Color::Red`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Red
    pub fn error(mut self, error: Color) -> Self {
        self.error = error;
        self
    }

    /// Overrides the [`Warn`] level color with the given color.
    ///
    /// The default color is [`Color::Yellow`].
    ///
    /// [`Warn`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Warn
    /// [`Color::Yellow`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Yellow
    pub fn warn(mut self, warn: Color) -> Self {
        self.warn = warn;
        self
    }

    /// Overrides the [`Info`] level color with the given color.
    ///
    /// The default color is [`Color::White`].
    ///
    /// [`Info`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Info
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    pub fn info(mut self, info: Color) -> Self {
        self.info = info;
        self
    }

    /// Overrides the [`Debug`] level color with the given color.
    ///
    /// The default color is [`Color::White`].
    ///
    /// [`Debug`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Debug
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    pub fn debug(mut self, debug: Color) -> Self {
        self.debug = debug;
        self
    }

    /// Overrides the [`Trace`] level color with the given color.
    ///
    /// The default color is [`Color::White`].
    ///
    /// [`Trace`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Trace
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    pub fn trace(mut self, trace: Color) -> Self {
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
    /// [`Error`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Error
    /// [`Warn`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Warn
    /// [`Info`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Info
    /// [`Debug`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Debug
    /// [`Trace`]: https://docs.rs/log/0.4/log/enum.Level.html#variant.Trace
    /// [`Color::White`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.White
    /// [`Color::Yellow`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Yellow
    /// [`Color::Red`]: https://docs.rs/colored/1/colored/enum.Color.html#variant.Red
    pub fn default() -> Self {
        ColoredLevelConfig {
            error: Color::Red,
            warn: Color::Yellow,
            debug: Color::White,
            info: Color::White,
            trace: Color::White,
        }
    }

    /// Colors the given log level with the color in this configuration corresponding to it's
    /// level.
    ///
    /// The structure returned is opaque, but will print the Level surrounded by ANSI color
    /// codes when displayed. This will work correctly for UNIX terminals, but due to a lack
    /// of support from the [`colored`] crate, this will not function in Windows.
    ///
    /// [`colored`]: https://github.com/mackwic/colored
    pub fn color(&self, level: Level) -> WithFgColor<Level> {
        level.colored(self.get_color(&level))
    }

    /// Retrieves the color that a log level should be colored as.
    pub fn get_color(&self, level: &Level) -> Color {
        match *level {
            Level::Error => self.error,
            Level::Warn => self.warn,
            Level::Info => self.info,
            Level::Debug => self.debug,
            Level::Trace => self.trace,
        }
    }
}

impl ColoredLogLevel for Level {
    fn colored(&self, color: Color) -> WithFgColor<Level> {
        WithFgColor {
            text: *self,
            color: color,
        }
    }
}

#[cfg(test)]
mod test {
    use super::WithFgColor;
    use colored::Colorize;
    use colored::Color::*;

    #[test]
    fn fg_color_matches_colored_behavior() {
        for &color in &[
            Black,
            Red,
            Green,
            Yellow,
            Blue,
            Magenta,
            Cyan,
            White,
            BrightBlack,
            BrightRed,
            BrightGreen,
            BrightYellow,
            BrightBlue,
            BrightMagenta,
            BrightCyan,
            BrightWhite,
        ] {
            assert_eq!(
                format!("{}", "test".color(color)),
                format!(
                    "{}",
                    WithFgColor {
                        text: "test",
                        color: color,
                    }
                )
            );
        }
    }
}
