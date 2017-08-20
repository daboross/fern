fern
====
[![Linux Build Status][travis-image]][travis-builds]
[![Windows Build Status][appveyor-image]][appveyor-builds]

Simple, efficient logging for [Rust].

---

Fern
====

Logging configuration is infinitely branched, like a fern: formatting, filters, and output are all controlled for any increasingly specific set of parameters. Fern provides a builder-based configuration and implementation for rust's standard [`log`] crate.

```rust
//! With fern, we can:

// Configure logger at runtime
fern::Dispatch::new()
    // Perform allocation-free log formatting
    .format(|out, message, record| {
        out.finish(format_args!("{}[{}][{}] {}",
            chrono::Local::now()
                .format("[%Y-%m-%d][%H:%M:%S]"),
            record.target(),
            record.level(),
            message))
    })
    // Add blanket level filter -
    .level(log::LogLevelFilter::Debug)
    // - and per-namespace overrides
    .level_for("hyper", log::LogLevelFilter::Info)
    // Output to stdout, files, and other Dispatch configs
    .chain(std::io::stdout())
    .chain(fern::log_file("output.log")?)
    // Apply globally
    .apply()?;

// and log using log crate macros!
info!("helllo, world!");
```

More contrived, and useful, examples at the [`api docs`][fern-docs] and the [example command line program][fern-example].

---

- [`fern` documentation][fern-docs]
- [`fern` on crates.io](crates.io/crates/fern/)
- [`fern` in use (example program)][fern-example]

### Project Status

The `fern` project, so far, has been maintained by myself alone. It's a hobby project, but one I aim to keep at a high quality now and in the future.

### Contributing

With that said, contributions are also welcome!

The easiest way for you to contribute right now is to use `fern` in your application, and see where it's lacking. The current library should have a solid base, but not many log adapters or niche features.

If you have a use case `fern` does not cover, filing an issue will be immensely useful to me, to anyone wanting to contribute to the project, and (hopefully) to you once the feature is implemented!

If you've just filed an issue, or you want to approach one of our [existing ones](https://github.com/daboross/fern/issues), mentoring is available! Tag me with @daboross on an issue, or send me an email at daboross @ daboross.net, and I'll be available to help.

See [CONTRIBUTING](./CONTRIBUTING.md) for more information on technical details.

[Rust]: https://www.rust-lang.org/
[travis-image]: https://travis-ci.org/daboross/fern.svg?branch=master
[travis-builds]: https://travis-ci.org/daboross/fern
[appveyor-image]: https://ci.appveyor.com/api/projects/status/github/daboross/fern?branch=master&svg=true
[appveyor-builds]: https://ci.appveyor.com/project/daboross/fern
[fern-docs]: https://dabo.guru/rust/fern/
[fern-example]: https://github.com/daboross/fern/tree/master/examples/cmd-program.rs
[`log`]: https://github.com/rust-lang-nursery/log
