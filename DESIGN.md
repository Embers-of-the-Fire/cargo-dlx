# CLI Api Design

This document lists some of the fundamental design of `cargo dlx` command.
Not all features listed below are implemented now.

Unresolved problems:

- relative importance of a `cargox` alias for `cargo dlx`
- how does a user update if they do `cargo dlx ripgrep` (no version)
- how much to mirror `cargo install`s behavior vs do its own thing (e.g. #7169)
- analysis of prior art and their relevance to Cargo
- whether to reintroduce `-c` shell execution

## Packages and Sources

`cargo dlx` is designed to support the standard [Package Id Specifications](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html) of Cargo.

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

## Argument Passing and Binary Calling

`cargo dlx` is designed to support the following calling:

1. Simple, direct forwarding: `cargo dlx [COMMAND ARGS] [<PACKAGE>] [PACKAGE ARGS]`
2. Calling with explicit binary: `cargo dlx [COMMAND ARGS] [<PACKAGE>] [COMMAND ARGS] -- [ANY COMMAND]`.

Format `1.` is roughly `cargo run -- [PACKAGE ARGS]`.

Format `2.` compiles and installs the package to a temporary directory,
and inject the binary path to `$PATH`, then execute the arbitrary command.

Implement status: Only `1.` is implemented now.

## Caching Strategy

`cargo dlx` would maintain a (configurable) global cache directory to compile a specific package.
The binary output is installed to another temporary directory to be executed, and is not cached.

## Garbage Collection Strategy

`cargo dlx` does not clean the temporary directory after run,
as some package may generate files within the working directory.

However, `cargo dlx` offers a `--clear` option to perform garbage collection.
This would delete all temporary directory and the global build cache.

Implement status: Not implemented now.

## Multiple Binaries

`cargo dlx` would support a `--bin`/`--example` for packages to specify the target binary to execute.
If direct forwarding is used and no binary is specified, an error will be generated.
If the user decided to use the explicit binary calling, a warning will be generated and all binaries would be compiled.

Implement status: Not implemented now.

## Profile

`cargo dlx` would offer a default `release` profile, and should be configurable via:

- CLI argument to specify a TOML configuration file.
- Global configuration file.

Implement status: Not implemented now.

## Package-Specific Configuration

`cargo dlx` would respect to user-defined package-specific configuration, but would not track them implicitly.
That is to say, `cargo dlx` would accept configuration written to its configuration files,
but would not automatically save them after a random call.

Implement status: Not implemented now.
