fern
====

fern is a runtime-configurable logging library written for [rust](http://www.rust-lang.org/).

Current features are:
- Multiple loggers. You can create as many loggers as you need, and configure them separately.
- Configurable output format via closures.
- Multiple outputs per logger - current options are to a file, or to stdout/stderr (or any combination of those)
- Each output can have a Level configured, so you can output all log messages to a log file, and only have warnings and above show up in the console!
- You can also define your own custom logging endpoints - have messages end up where you need them!
- Thread-local logger storage. This allows for convenient <level>!() and log!() macros via the fern_macros package.

fern is still in development, and most features are experimental. The library is subject to change in non-backwards-compatible ways.

This library can only be used while complying to the license terms in the `LICENSE` file.
