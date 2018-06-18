/*!
Example usage of `fern` with the `syslog` crate.

Be sure to depend on `syslog` and the `syslog` feature in `Cargo.toml`:

```toml
[dependencies]
fern = { version = "0.5", features = ["syslog-3"] }]
syslog = "3.3"
```

To use `syslog`, simply create the log you want, and pass it into `Dispatch::chain`:

```no_run
extern crate fern;
extern crate syslog;

# fn setup_logging() -> Result<(), fern::InitError> {
fern::Dispatch::new()
    .chain(syslog::unix(syslog::Facility::LOG_USER)?)
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

One thing with `syslog` is that you don't generally want to apply any log formatting. The system
logger will handle that for you.

However, you probably will want to format messages you also send to stdout! Fortunately, selective
configuration is easy with fern:

```no_run
# extern crate fern;
# extern crate log;
# extern crate syslog;
#
# fn setup_logging() -> Result<(), fern::InitError> {
// top level config
fern::Dispatch::new()
    .chain(
        // console config
        fern::Dispatch::new()
            .level(log::LevelFilter::Debug)
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{}] {}",
                    record.level(),
                    message,
                ))
            })
            .chain(std::io::stdout())
    )
    .chain(
        // syslog config
        fern::Dispatch::new()
            .level(log::LevelFilter::Info)
            .chain(syslog::unix(syslog::Facility::LOG_USER)?)
    )
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

With this, all info and above messages will be sent to the syslog with no formatting, and
the messages sent to the console will still look nice as usual.

---

One last pattern you might want to know: creating a log target which must be explicitly mentioned
in order to work.

```no_run
# extern crate fern;
# extern crate log;
# extern crate syslog;
#
# fn setup_logging() -> Result<(), fern::InitError> {
fern::Dispatch::new()
    // by default only accept warning messages from libraries so we don't spam
    .level(log::LevelFilter::Warn)
    // but accept Info and Debug if we explicitly mention syslog
    .level_for("explicit-syslog", log::LevelFilter::Debug)
    .chain(syslog::unix(syslog::Facility::LOG_USER)?)
    .apply()?;
# Ok(())
# }
# fn main() { setup_logging().ok(); }
```

With this configuration, only warning messages will get through by default. If we do want to
send info or debug messages, we can do so explicitly:

```no_run
# #[macro_use]
# extern crate log;
# fn main() {
debug!("this won't get through");
// especially useful if this is from library you depend on.
info!("neither will this");
warn!("this will!");

info!(target: "explicit-syslog", "this will also show up!");
# }
```
*/
