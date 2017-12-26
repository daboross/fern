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
        out.finish(format_args!(
            "{}[{}][{}] {}",
            chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
            record.target(),
            record.level(),
            message
        ))
    })
    // Add blanket level filter -
    .level(log::LevelFilter::Debug)
    // - and per-namespace overrides
    .level_for("hyper", log::LevelFilter::Info)
    // Output to stdout, files, and other Dispatch configurations
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

The `fern` project is primarily maintained by @daboross on github. It's a hobby project, but one I aim to keep at a high quality.

### Contributing

As this is a hobby project, contributions are very welcome!

The easiest way for you to contribute right now is to use `fern` in your application, and see where it's lacking. The current library has a solid base, but it lacks features, and I may not anticipate your use cases.

If you have a use case `fern` does not cover, please file an issue. This is immensely useful to me, to anyone wanting to contribute to the project, and to you as well if the feature is implemented.

If you're interested in helping fix an [existing issue](https://github.com/daboross/fern/issues), or an issue you just filed, help is always welcome.

See [CONTRIBUTING](./CONTRIBUTING.md) for technical information on contrbuting.

[Rust]: https://www.rust-lang.org/
[travis-image]: https://travis-ci.org/daboross/fern.svg?branch=master
[travis-builds]: https://travis-ci.org/daboross/fern
[appveyor-image]: https://ci.appveyor.com/api/projects/status/github/daboross/fern?branch=master&svg=true
[appveyor-builds]: https://ci.appveyor.com/project/daboross/fern
[fern-docs]: https://dabo.guru/rust/fern/
[fern-example]: https://github.com/daboross/fern/tree/master/examples/cmd-program.rs
[`log`]: https://github.com/rust-lang-nursery/log
