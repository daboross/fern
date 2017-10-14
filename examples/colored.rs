extern crate fern;
extern crate colored;
#[macro_use]
extern crate log;
extern crate chrono;

use colored::Color;
use fern::colors::ColoredLogLevel;
use log::LogLevel;

fn main() {
    fern::Dispatch::new()
        .chain(std::io::stdout())
        .color(LogLevel::Error, Color::Black)
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}]{} {}",
                record.level().colored(),
                chrono::Utc::now().format("[%Y-%m-%d %H:%M:%S]"),
                message
            ))
        })
        .apply()
        .unwrap();

    error!("hi");
    debug!("sup");
    warn!("oh");
}