fern
====

fern is a runtime-configurable logging library written for [rust](http://www.rust-lang.org/).

Current features are:
- Multiple loggers. You can create as many loggers as you need, and configure them separately.
- Configurable output format via closures.
- Multiple outputs per logger - output to any combination of:
  - log files
  - stdout or stderr
  - your own custom implementation
- Each output can have a Level configured, so you can output all log messages to a log file,
  and only have warnings and above show up in the console.
- You can also define your own custom logging endpoints - have messages end up where you need
  them.
- Acts as a backend to the `log` crate - use `trace!()` through `error!()` on your main logger
  - Note that fern can also have loggers separate from the `log` crate's global system. It's
    possible to just set your main logger as the global logger, then use other ones manually
    as well.

Although mostly stabilized, fern is still in development. The library is subject to
change in non-backwards-compatible ways before the API is completely stabilized.

This library can only be used while complying to the license terms in the `LICENSE` file.

The more information, and examples on how to use fern, see [the fern docs](https://dabo.guru/rust/fern/).
