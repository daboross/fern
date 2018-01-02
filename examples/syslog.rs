extern crate fern;
#[macro_use]
extern crate log;
extern crate syslog;

fn setup_logging() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        // by default only accept warning messages from libraries so as not to spam
        .level(log::LevelFilter::Warn)
        // but accept Info if we explicitly mention it
        .level_for("explicit-syslog", log::LevelFilter::Info)
        .chain(syslog::unix(syslog::Facility::LOG_USER)?)
        .apply()?;

    Ok(())
}

fn main() {
    setup_logging().expect("failed to initialize logging.");

    // Emulate a library we're using which has tons of debugging on the 'info' level.
    info!(target: "overly-verbose-target", "hey, another library here, we're starting.");

    for i in 0..5 {
        info!("executing section: {}", i);

        debug!("section {} 1/4 complete.", i);

        info!(target: "overly-verbose-target", "completed operation.");

        debug!("section {} 1/2 complete.", i);

        info!(target: "overly-verbose-target", "completed operation.");

        info!(target: "overly-verbose-target", "completed operation.");

        debug!("section {} 3/4 complete.", i);

        info!("section {} completed!", i);
    }

    info!(target: "explicit-syslog-info", "hello to the syslog! this is rust.");

    warn!(target: "overly-verbose-target", "AHHH something's on fire.");
}
