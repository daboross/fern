Fern: efficient, configurable logging in rust
====
[![Linux Build Status][travis-image]][travis-builds]
[![Windows Build Status][appveyor-image]][appveyor-builds]

With fern, you can:

- Configure logging at runtime; make changes based off of user arguments or configuration
- Format log records without allocating intermediate results
- Output to stdout, stderr, log files and custom destinations
- Apply a blanket level filter and per-crate/per-module overrides
- Intuitively apply filters and formats to groups of loggers via builder chaining
- Log using the standard `log` crate macros

API Docs: https://dabo.guru/rust/fern-dev/fern/

Full example program: https://github.com/daboross/fern/tree/master/examples/cmd-program.rs

Stability warning:

Fern, while feature-complete, does not have a mature API. The library may be changed
in backwards incompatible ways to make it more ergonomic in the future.

This library can only be used while complying to the license terms in the `LICENSE` file.

The more information, and examples on how to use fern, see [the fern docs](https://dabo.guru/rust/fern/).

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

[travis-image]: https://travis-ci.org/daboross/fern.svg?branch=master
[travis-builds]: https://travis-ci.org/daboross/fern
[appveyor-image]: https://ci.appveyor.com/api/projects/status/github/daboross/fern?branch=master&svg=true
[appveyor-builds]: https://ci.appveyor.com/project/daboross/fern
