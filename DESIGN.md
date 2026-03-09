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

- `foo` / `foo@version` (crates.io)
- `git+<URL>[?branch=...|tag=...|rev=...][#<pkg>[@<ver>] | #<ver>]`
- `file:///<absolute-path>[#<pkg>[@<ver>] | #<ver>]`
- `path+file:///<absolute-path>[#<pkg>[@<ver>] | #<ver>]`
- `registry+<index-url>#<pkg>[@<ver>]`
- `sparse+<index-url>#<pkg>[@<ver>]`

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

`cargo dlx` maintains a global runtime root, similar to Cargo's `~/.cargo` layout.

Default root:

- `CARGO_DLX_ROOT`, or fallback `~/.cargo-dlx`

Directory layout under the root:

- `tmp/<timestamp>`: per-run installation runtime root (ephemeral)
- `build/target`: Cargo build cache directory (`CARGO_TARGET_DIR`)

Overrides:

- `CARGO_DLX_TEMP`: overrides the temp runtime base directory (`tmp`)
- `CARGO_DLX_BUILD`: overrides the build cache base directory (`build`)

CLI overrides:

- `--cache-dir <DIR>` can still override the Cargo build target directory directly.

The installed runnable binaries remain ephemeral and are not cached between invocations.

## Garbage Collection Strategy

Current behavior:

- `tmp/<timestamp>` installation roots are removed automatically when the process exits.
- build cache (`build/target`) remains for reuse across invocations.
- `--clear` removes temporary install roots and package cache directories.

## Multiple Binaries

`cargo dlx` would support a `--bin`/`--example` for packages to specify the target binary to execute.
If direct forwarding is used and no binary is specified, an error will be generated.
If the user decided to use the explicit binary calling, a warning will be generated and all binaries would be compiled.

Implement status: Not implemented now.

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
