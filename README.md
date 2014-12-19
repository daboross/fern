fern
====

fern is a configurable logging framework written for [rust](http://www.rust-lang.org/).

Current features are:
- Multiple loggers. You can create as many loggers as you need, and configure them separately.
- Configurable output format via closures.
- Multiple outputs per logger - current options are to a file, or to stdout/stderr (or any combination of those)
 - Each output can have a Level configured, so you can output all debug messages to a debug.log, while having only info and above go to regular.log - and then reserve stderr for all severe error messages!

fern is still in development, and most features are experimental. The library is subject to change in non-backwards-compatible ways.

