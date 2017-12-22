extern crate fern;
#[macro_use]
extern crate log;
extern crate chrono;

use fern::colors::{Color, ColoredLogLevelConfig};

fn main() {
    let mut config = ColoredLogLevelConfig::default();
    config.debug = Color::Magenta;

    fern::Dispatch::new()
        .chain(std::io::stdout())
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}]{} {}",
                config.color(record.level()),
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
