fern
====
[![crates.io version badge][cratesio-badge]][fern-crate]
[![Build Status][test-status-badge]][test-status-link]

- [documentation][fern-docs]
- [crates.io page][fern-crate]
- [example program][fern-example]

Simple, efficient logging for [Rust].

Logging configuration is recursively branched: formatting, filters, and output can be applied at each
`fern::Dispatch`, applying to increasingly specific kinds of logging.

```rust
// Configure logger at runtime
fern::Dispatch::new()
    // Perform allocation-free log formatting
    .format(|out, message, record| {
        out.finish(format_args!(
            "[{} {} {}] {}",
            humantime::format_rfc3339(std::time::SystemTime::now()),
            record.level(),
            record.target(),
            message
        ))
    })
    // Add blanket level filter -
    .level(log::LevelFilter::Debug)
    // - and per-module overrides
    .level_for("hyper", log::LevelFilter::Info)
    // Output to stdout, files, and other Dispatch configurations
    .chain(std::io::stdout())
    .chain(fern::log_file("output.log")?)
    // Apply globally
    .apply()?;

// and log using log crate macros!
log::info!("hello, world!");
```

Examples of all features at the [api docs][fern-docs]. See fern in use with this [example command line program][fern-example].

## Project Direction

I've posted a GitHub Discussion talking about the future of fern: https://github.com/daboross/fern/discussions/147

If you've ever used fern, or you do today, I'd love input!

## fern 0.4.4, 0.5.\*, 0.6.\* security warning - `colored` crate + custom global allocator

One of our downstream dependencies, [atty](https://docs.rs/atty/), through
[colored](https://docs.rs/colored/), has an unsoundness issue:
<https://rustsec.org/advisories/RUSTSEC-2021-0145.html>.

This shows up in one situation: if you're using `colored` 0.1.0 and a custom global allocator.

Upgrade to `fern` 0.7.0 to fix.

### Contributing

There's one thing I need right now, more than anything else: input on what fern does well, and what it should keep
doing well. See [Project Direction](#project-direction).

Besides that, I'm open to PRs! I'll probably review promptly, and I'm always open to being nudged if I don't.

For small PRs, I'll mark anything I need changed in a review, and work with you on that.

For larger PRs, I reserve the right to pull in your commits as they are, then fix things I want to be different myself.
In a workplace, I'd try to never do this - but this is a hobby project for me, and I'd rather be overly particular about
fern's implementation than be reasonable.

This is a change from my previous policy.

See [CONTRIBUTING](./CONTRIBUTING.md) for technical information on contributing.

[Rust]: https://www.rust-lang.org/
[test-status-badge]: https://github.com/daboross/fern/workflows/tests/badge.svg?branch=main&event=push
[test-status-link]: https://github.com/daboross/fern/actions/workflows/rust.yml
[issue-resolution-badge]: http://isitmaintained.com/badge/resolution/daboross/fern.svg
[isitmaintained-link]: http://isitmaintained.com/project/daboross/fern
[cratesio-badge]: https://img.shields.io/crates/v/fern.svg
[fern-docs]: https://docs.rs/fern/
[fern-crate]: https://crates.io/crates/fern
[fern-example]: https://github.com/daboross/fern/tree/main/examples/cmd-program.rs
[log]: https://github.com/rust-lang/log
