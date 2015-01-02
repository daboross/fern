use std::fmt;
use std::sync;

use errors::Error;

/// Basic logger trait. Something you can send messages to.
#[unstable]
pub trait Logger {
    /// Logs a given message - puts this message where-ever this logger means to put it
    #[unstable]
    fn log(&self, level: &Level, message: &str) -> Result<(), Error>;
}

/// Type alias for a boxed logger
#[unstable]
pub type BoxedLogger = Box<Logger + Sync + Send>;

/// Type alias for a reference counted boxed logger
#[unstable]
pub type ArcLogger = sync::Arc<Box<Logger + Sync + Send>>;

/// A logging level - definition of how severe your message is.
#[unstable]
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
    pub fn as_int(&self) -> u8 {
        match self {
            &Level::Debug => 0u8,
            &Level::Info => 1u8,
            &Level::Warning => 2u8,
            &Level::Severe => 3u8,
        }
    }
}

#[unstable]
impl fmt::Show for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}", match self {
            &Level::Debug => "DEBUG",
            &Level::Info => "INFO",
            &Level::Warning => "WARNING",
            &Level::Severe => "SEVERE",
        });
    }
}
