use log;

use errors::LogError;

/// Basic fern logger trait. Something you can send messages to. We have a separate trait from
/// log::Log, because we want errors to propagate upwards and only print in the outermost logger.
pub trait Logger: Sync + Send {
    /// Logs a given message in this logger.
    fn log(&self, msg: &str, level: &log::LogLevel, location: &log::LogLocation)
            -> Result<(), LogError>;
}

impl Logger for Box<Logger> {
    fn log(&self, msg: &str, level: &log::LogLevel, location: &log::LogLocation)
            -> Result<(), LogError> {
        (**self).log(msg, level, location)
    }
}
