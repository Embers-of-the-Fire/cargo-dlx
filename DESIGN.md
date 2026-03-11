# CLI Design

This document lists some of the fundamental design of `cargo dlx` command.
Not all features listed below are implemented now.

## Goal

This is a polyfill for what could be merged into Cargo.
Design decisions should align with existing design elements in Cargo.

## Open questions

- relative importance of a `cargox` alias for `cargo dlx`
- how does a user update if they do `cargo dlx ripgrep` (no version)
- how much to mirror `cargo install`s behavior vs do its own thing (e.g. #7169)
- analysis of prior art and their relevance to Cargo
- whether to reintroduce `-c` shell execution

## Specifying the package to run

To allow specifying packages from any dependency source,
`cargo dlx` accepts Cargo's
[Package Id Specifications](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html) format.

Implement status:

- ✅ `foo` / `foo@version` (crates.io)
- ✅ `git+<URL>[?branch=...|tag=...|rev=...][#<pkg>[@<ver>] | #<ver>]`
- ✅ `file:///<absolute-path>[#<pkg>[@<ver>] | #<ver>]`
- ✅ `path+file:///<absolute-path>[#<pkg>[@<ver>] | #<ver>]`
- ✅ `registry+<index-url>#<pkg>[@<ver>]`
- ✅ `sparse+<index-url>#<pkg>[@<ver>]`

Behavior notes for current implementation:

- For `git+` references, query parameters are translated to `cargo install --branch/--tag/--rev`.
- For `file://` and `path+file://` references, Cargo is invoked with `cargo install --path`.
- Local `file://` URLs are absolute-path only.
- For `registry+` / `sparse+` references, Cargo is invoked with `cargo install --index`.
- Registry references must include the package in the URL fragment (`#my-crate`).
- If a git/path fragment contains only a version (`#1.2.3`), package name is inferred from the source path basename when possible.

Alternatives:
- Only accept `<name>[@<ver>]` from the registry
- Have `<name>` pull from a local `Cargo.lock` like `cargo info`

## Forwarding arguments

Arguments must be forwarded to the underlying binary in a clear and unambiguous manner.

Common challenges include:
- Overlapping flags between `cargo dlx` and the binary, especially `--help`

Once `cargo dlx` parses the package,
all following arguments are captured for forwarding to the specified binary.

The following are roughly equivalent if `cargo run` could work with arbitrary packages:
```console
$ cargo dlx [DLX_ARGS] <PACKAGE> [PACKAGE_ARGS]
$ cargo run -p <PACKAGE> [DLX_ARGS] -- [PACKAGE_ARGS]
```

## Multi-binary packages

While most packages have just one binary,
that isn't an inherent requirement.
In addition, users may wish to run an example.

`cargo dlx` will use the Cargo standard `--bin` and `--example` arguments to specify a specific binary to build and run.

Cargo semantics:
- a single `[[bin]]` is considered the default
- if multiple `[[bin]]`s are present, `package.default-run` can specify the default
- if there are multiple `[[bin]]`s without a default, error and list all `[[bin]]`s
- `--bin` or `--example` without a name lists available names

Implement status: Not implemented now.

Alternatives:
- Have a syntax to mix this in with the package selection
- If a [`last`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.last) argument is present, the usage becomes `cargo dlx [DLX_ARGS] <PACKAGE> <BIN> -- [PACKAGE_ARGS]`

## Caching strategy

Users want
- performance: repeated calls to `cargo dlx foo` doing the minimal work possible
- parallelism: what happens when two `cargo dlx` calls are run in parallel
- disk space: least used
- upgrades: getting newer versions of under-specified package versions
- compiler: getting the benefits of the latest compiler
- settings: able to specify the binary, features, profile, etc

These are inherently contradictory.

`cargo dlx` builds in a cross-package `target-dir` and installs to an ephemeral location.
- performance: repeated calls leverage the fingerprint which has some overhead that scales with application complexity
- parallelism: builds, even no-op builds, block on each other
- disk space: large intermediate build artifacts are retained but sharing is done between packages and settings
- upgrades: latest version is always used
- compiler: current compiler is always used
- settings: changing a setting only rebuilds as much as is needed

Alternatives:
- Per-package `target-dir`s
  - performance: unchanged
  - parallelism: only blocking between runs of the same package
  - disk space: no reuse between different packages but reuse is likely limited always
  - upgrades: unchanged
  - compiler: unchanged
  - settings: unchanged
- Use an ephemeral `target-dir`, installing into a location under a hash of the `dlx` inputs
  - performance: repeated have a small constant overhead
  - parallelism: no blocking between unique dlx inputs
  - disk space: no extra disk space is used
  - upgrades: mechanism is need to request an upgrade
  - compiler: mechanism is need to request a rebuild
  - settings: changing a setting causes a full rebuild

## Garbage collection strategy

As part of using the least disk space possible,
there needs to be a way to clean up binaries that are no longer used.

With the current caching strategy,
there are two cache locations:
- the ephemeral install location: auto-cleaned up on completion
- `target-dir`: deferred to [rust-lang/cargo#5026](https://github.com/rust-lang/cargo/issues/5026)

## Profile

`cargo dlx` defaults to the `release` profile, and is configurable via:

- ✅ `--profile`

Whether the default can be overriden in a config file is dependent on feedback,
including gathering use cases for it.

## Package-Specific Configuration

`cargo dlx` would respect to user-defined package-specific configuration, but would not track them implicitly.
That is to say, `cargo dlx` would accept configuration written to its configuration files,
but would not automatically save them after a random call.

Implement status: Not implemented now.
