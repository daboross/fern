use colored::{Color, ColoredString, Colorize};
use log::LogLevel;
use builders::Dispatch;
use std::sync::Mutex;

lazy_static! {
    pub static ref ERROR_COLOR: Mutex<Color> = Mutex::new(Color::Red);
    pub static ref WARN_COLOR: Mutex<Color> = Mutex::new(Color::Yellow);
    pub static ref INFO_COLOR: Mutex<Color> = Mutex::new(Color::Cyan);
    pub static ref DEBUG_COLOR: Mutex<Color> = Mutex::new(Color::Green);
    pub static ref TRACE_COLOR: Mutex<Color> = Mutex::new(Color::White);
}

impl Dispatch {
    #[inline]
    pub fn color(self, level: LogLevel, color: Color) -> Self {
        match level {
            LogLevel::Error => {
                let mut state = ERROR_COLOR.lock().unwrap();

                *state = color
            },
            LogLevel::Warn => {
                let mut state = WARN_COLOR.lock().unwrap();

                *state = color
            },
            LogLevel::Info => {
                let mut state = INFO_COLOR.lock().unwrap();

                *state = color
            },
            LogLevel::Debug => {
                let mut state = DEBUG_COLOR.lock().unwrap();

                *state = color
            },
            LogLevel::Trace => {
                let mut state = TRACE_COLOR.lock().unwrap();

                *state = color
            },
        }

        self
    }
}

pub trait ColoredLogLevel {
    fn colored(&self) -> ColoredString;
    fn as_colored(&self, color: Color) -> ColoredString;
}

fn new_colored_string(input: String, color: Color) -> ColoredString {
    let cs = ColoredString::from(input.as_str());

    cs.color(color)
}

fn get_color(level: &LogLevel) -> Color {
    match *level {
        LogLevel::Error => *ERROR_COLOR.lock().unwrap(),
        LogLevel::Warn => *WARN_COLOR.lock().unwrap(),
        LogLevel::Info => *INFO_COLOR.lock().unwrap(),
        LogLevel::Debug => *DEBUG_COLOR.lock().unwrap(),
        LogLevel::Trace => *TRACE_COLOR.lock().unwrap(),
    }
}

impl ColoredLogLevel for LogLevel {
    fn colored(&self) -> ColoredString {
        let s = format!("{:?}", self);

        new_colored_string(s, get_color(self))
    }

    fn as_colored(&self, color: Color) -> ColoredString {
        let s = format!("{:?}", self);

        new_colored_string(s, color)
    }
}