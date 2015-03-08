use std::fmt;
use std::cmp;

use errors::LogError;

/// Basic fern logger trait. Something you can send messages to.
#[unstable]
pub trait Logger: Sync + Send {
    /// Logs a given message - puts this message where ever this logger means to put it
    #[unstable]
    fn log(&self, message: &str, level: &Level) -> Result<(), LogError>;
}

impl Logger for Box<Logger> {
    fn log(&self, msg: &str, level: &Level) -> Result<(), LogError> {
        (**self).log(msg, level)
    }
}

/// A logging level - definition of how severe your message is.
#[unstable]
#[derive(Eq, PartialEq)]
pub enum Level {
    /// Debug - things that the end user probably won't want to see
    #[unstable]
    Debug,
    // Info level - just regular old messages, normal operation
    #[unstable]
    Info,
    // Warning level - something didn't go to plan, but the application isn't going to stop working
    #[unstable]
    Warning,
    // Severe level - something went really wrong, you *really* need to pay attention
    #[unstable]
    Severe,
}

#[stable]
impl Copy for Level {}

#[unstable]
impl Level {
    /// Take the integer value of this log level. This value is basically 0-3, increasing as the
    /// severity of the log increases.
    #[inline]
    #[unstable]
    fn as_int(&self) -> u8 {
        match self {
            &Level::Debug => 0u8,
            &Level::Info => 1u8,
            &Level::Warning => 2u8,
            &Level::Severe => 3u8,
        }
    }
}

#[unstable]
impl cmp::PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        return Some(self.as_int().cmp(&other.as_int()));
    }

    fn lt(&self, other: &Self) -> bool {
        return self.as_int() < other.as_int();
    }
    fn le(&self, other: &Self) -> bool {
        return self.as_int() <= other.as_int();
    }
    fn gt(&self, other: &Self) -> bool {
        return self.as_int() > other.as_int();
    }
    fn ge(&self, other: &Self) -> bool {
        return self.as_int() >= other.as_int();
    }
}

#[unstable]
impl cmp::Ord for Level {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        return self.as_int().cmp(&other.as_int());
    }
}

#[unstable]
impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return f.write_str(match self {
            &Level::Debug => "DEBUG",
            &Level::Info => "INFO",
            &Level::Warning => "WARNING",
            &Level::Severe => "SEVERE",
        });
    }
}
