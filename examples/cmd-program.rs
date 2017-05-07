extern crate fern;
#[macro_use]
extern crate log;
extern crate chrono;

use std::{io, env};

fn setup_logging(verbose: bool) -> Result<(), fern::InitError> {
    let mut base_config = fern::Dispatch::new().level(if verbose {
        log::LogLevelFilter::Debug
    } else {
        log::LogLevelFilter::Info
    });

    if !verbose {
        // Let's say we depend on something which whose "info" level messages are too verbose
        // to include in end-user output. If we don't need them, let's not include them.
        base_config = base_config.level_for("overly-verbose-target", log::LogLevelFilter::Warn);
    }

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("{}[{}][{}] {}",
                                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                                    record.target(),
                                    record.level(),
                                    message))
        })
        .chain(fern::log_file("program.log")?);

    let stdout_config = fern::Dispatch::new()
        .format(|out, message, record| {
            // special format for debug messages coming from our own crate.
            if record.level() > log::LogLevelFilter::Info && record.target() == "cmd_program" {
                out.finish(format_args!("---\nDEBUG: {}: {}\n---",
                                        chrono::Local::now().format("%H:%M:%S"),
                                        message))
            } else {
                out.finish(format_args!("[{}][{}][{}] {}",
                                        chrono::Local::now().format("%H:%M"),
                                        record.target(),
                                        record.level(),
                                        message))
            }
        })
        .chain(io::stdout());

    base_config.chain(file_config).chain(stdout_config).into_global_logger()?;

    Ok(())
}

fn main() {
    let verbose = env::args().any(|arg| arg == "-v" || arg == "--verbose");

    setup_logging(verbose).expect("failed to initialize logging.");

    info!("MyProgram v0.0.1 starting up!");

    if verbose {
        info!("DEBUG output enabled.")
    }

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

    warn!(target: "overly-verbose-target", "AHHH something's on fire.");

    info!("MyProgram operation completed, shutting down.");
}
