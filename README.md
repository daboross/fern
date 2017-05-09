fern
====
[![Linux Build Status][travis-image]][travis-builds]
[![Windows Build Status][appveyor-image]][appveyor-builds]

Simple, efficient logging for [Rust].

---

With fern, you can:

- Configure logging at runtime; make changes based off of user arguments or configuration
- Format log records without allocating intermediate results
- Output to stdout, stderr, log files and custom destinations
- Apply a blanket level filter and per-crate/per-module overrides
- Intuitively apply filters and formats to groups of loggers via builder chaining
- Log using the standard `log` crate macros

---

- [`fern` documentation](https://dabo.guru/rust/fern/)
- [`fern` on crates.io](crates.io/crates/fern/)
- [`fern` in use (example program)](https://github.com/daboross/fern/tree/master/examples/cmd-program.rs)

### Testing

Fern has two separate tests which both require initializing the global logger, so the tests must be run separately. To test, use:

```sh
cargo test -- --skip test2
cargo test test2
```

To run the example program, use:

```sh
cargo run --example cmd-program
cargo run --example cmd-program -- --verbose
```

[Rust]: https://www.rust-lang.org/
[travis-image]: https://travis-ci.org/daboross/fern.svg?branch=master
[travis-builds]: https://travis-ci.org/daboross/fern
[appveyor-image]: https://ci.appveyor.com/api/projects/status/github/daboross/fern?branch=master&svg=true
[appveyor-builds]: https://ci.appveyor.com/project/daboross/fern
