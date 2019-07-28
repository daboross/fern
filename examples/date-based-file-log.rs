use log::{debug, info, warn};

fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    fern::Dispatch::new()
        // by default only accept warning messages so as not to spam
        .level(log::LevelFilter::Debug)
        .chain(fern::DateBasedLogFile::date_based("program.log"))
        .apply()?;

    Ok(())
}

fn main() {
    setup_logging().expect("failed to initialize logging.");

    // None of this will be shown in the syslog:
    for i in 0..5 {
        info!("executing section: {}", i);

        debug!("section {} 1/4 complete.", i);

        debug!("section {} 1/2 complete.", i);

        debug!("section {} 3/4 complete.", i);

        info!("section {} completed!", i);
    }

    // these two *will* show.

    info!(target: "explicit-syslog", "hello to the syslog! this is rust.");

    warn!("AHHH something's on fire.");
}
