use colored::{Color, ColoredString, Colorize};
use log::LogLevel;

pub trait ColoredLogLevel {
    fn colored(&self, color: Color) -> ColoredString;
}

fn new_colored_string(input: String, color: Color) -> ColoredString {
    let cs = ColoredString::from(input.as_str());
    cs.color(color)
}

#[derive(Copy, Clone)]
pub struct ColoredLogLevelConfig {
    pub trace: Color,
    pub error: Color,
    pub warn: Color,
    pub debug: Color,
    pub info: Color,
}

impl ColoredLogLevelConfig {
    pub fn new(trace: Color, error: Color, warn: Color, debug: Color, info: Color) -> Self {
        ColoredLogLevelConfig {
            trace: trace,
            error: error,
            warn: warn,
            debug: debug,
            info: info,
        }
    }

    pub fn default() -> Self {
        ColoredLogLevelConfig {
            trace: Color::White,
            error: Color::Red,
            warn: Color::Yellow,
            debug: Color::White,
            info: Color::White,
        }
    }

    pub fn color(&self, level: LogLevel) -> ColoredString {
        level.colored(self.get_color(&level))
    }

    fn get_color(&self, level: &LogLevel) -> Color {
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