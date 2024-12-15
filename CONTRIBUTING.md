# Contributing


## Overview (mirrored in README)

There's one thing I need right now, more than anything else: input on what fern does well, and what it should keep
doing well. See [Project Direction](#project-direction).

Besides that, I'm open to PRs! I'll probably review promptly, and I'm always open to being nudged if I don't.

For small PRs, I'll mark anything I need changed in a review, and work with you on that.

For larger PRs, I reserve the right to pull in your commits as they are, then fix things I want to be different myself.
In a workplace, I'd try to never do this - but this is a hobby project for me, and I'd rather be overly particular about
fern's implementation than be reasonable.

This is a change from my previous policy.

## Code of Conduct.

All interactions are expected to follow [the Rust Code of Conduct](https://www.rust-lang.org/en-US/conduct.html).

## `fern` project structure

Fern attempts to be an idiomatic rust library and to maintain a sane structure. All source code is located in `src/`, and tests are in `tests/`.

The source is split into four modules:
- `lib.rs` contains top-level traits, module documentation, and helper functions
- `builders.rs` contains all the configuration code
- `errors.rs` contains error handling for finishing configuration
- and `log_impl.rs` contains the implementation for `log::Log` which is created to run for the actual logging.

Hopefully these modules are fairly separated, and it's clear when you'll need to work on multiple sections. Adding a new log implementation, for instance, will need to touch `builders.rs` for configuration, and `log_impl.rs` for the implementation - both pieces of code will connect via `builders::Dispatch::into_dispatch`, but besides that, things should be fairly separate.

## Pull requests

Pull requests are _the_ way to change code using git. If you aren't familiar with them in general, GitHub has some [excellent documentation](https://help.github.com/articles/about-pull-requests/).

There aren't many hard guidelines in this repository on how specifically to format your request. Main points:

- Please include a descriptive title for your pull request, and elaborate on what's changed in the description.
- Feel free to open a PR before the feature is completely ready, and commit directly to the PR branch.
- Please include at least a short description in each commit, and more of one in the "main" feature commit. Doesn't
  have to be much, but someone reading the history should easily tell what's different now from before.
- Use `cargo fmt` to format your code.

## Testing

To run build everything and run all tests, use:

```sh
cargo build --all-features --all-targets
cargo test --all-features
```

## Mentoring

Contributing to a project can be daunting.

Email me at daboross @ daboross.net with any questions!
