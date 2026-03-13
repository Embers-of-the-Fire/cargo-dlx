# `cargo dlx`

`cargo dlx` downloads, compiles, and executes a Rust binary without installing it globally.

This package is in its early stages of development, and the API may change without a major version bump.

For design of the command, please refer to [DESIGN.md](DESIGN.md).
Feel free to discuss about design by creating a Pull Request against the document.

## Usage

```bash
cargo dlx ripgrep@14.1.1 --help
```

## Package Specification Syntax

`cargo dlx` accepts both simple package names and source-qualified package references.

Supported forms:

- `name`
- `name@version`
- `git+<url>[?branch=<name>|tag=<name>|rev=<sha>][#<pkg>[@<ver>] | #<ver>]`
- `file://<path>[#<pkg>[@<ver>] | #<ver>]`
- `file+file://<path>[#<pkg>[@<ver>] | #<ver>]`
- `path+file://<path>[#<pkg>[@<ver>] | #<ver>]`
- `registry+<index-url>#<pkg>[@<ver>]`
- `sparse+<index-url>#<pkg>[@<ver>]`

### Examples

Registry / crates.io:

```bash
cargo dlx ripgrep
cargo dlx ripgrep@14.1.1
cargo dlx 'registry+https://github.com/rust-lang/crates.io-index#ripgrep@14.1.1'
cargo dlx 'sparse+https://index.crates.io/#ripgrep@14.1.1'
```

Git:

```bash
cargo dlx 'git+https://github.com/rust-lang/cargo.git#cargo@0.85.0'
cargo dlx 'git+https://github.com/rust-lang/cargo.git?rev=<commit>#cargo'
cargo dlx 'git+https://github.com/owner/repo.git#1.2.3'
```

Local path (`file://`, `file+file://`, `path+file://`):

```bash
cargo dlx 'file:///absolute/path/to/my-tool#my-tool'
cargo dlx 'path+file:///absolute/path/to/my-tool#my-tool@0.1.0'
cargo dlx 'file+file:///absolute/path/to/my-tool#my-tool'
```

### Notes

- For `registry+...` / `sparse+...`, package name in fragment (`#pkg`) is required.
- For local file source references, URL query parameters are not supported.
- Local `file://` URLs must be absolute paths (for example, `file:///path/to/crate`).
- If you use shell-sensitive characters (like `?` or `#`), quote the whole package spec.

The command installs into a temporary directory, runs the binary, then removes the temporary install root.
`cargo dlx --clear` can be used to remove cached package artifacts and any stale temporary roots.
`cargo-dlx` invokes the Cargo executable from `$CARGO` when set, otherwise `cargo` from `PATH`.
Package build artifacts are cached in a persistent Cargo target directory (default under `~/.cargo-dlx/build/target`)
to speed up repeated runs, while installed binaries stay temporary.

## Cache Strategy

`cargo-dlx` caches **package build artifacts** only (via `CARGO_TARGET_DIR`) to speed up repeated runs.
It does **not** cache installed runnable binaries between invocations.

Default runtime layout (under `$CARGO_DLX_ROOT`, which defaults to `~/.cargo-dlx`):

- Temporary install roots: `$CARGO_DLX_ROOT/tmp/<timestamp>`
- Build cache target directory: `$CARGO_DLX_ROOT/build/target`

Build cache target directory selection (highest priority first):

1. `--cache-dir <DIR>`
2. `CARGO_DLX_BUILD`
3. `CARGO_DLX_ROOT/build`

Temporary install base directory selection (highest priority first):

1. `CARGO_DLX_TEMP`
2. `CARGO_DLX_ROOT/tmp`

## Storage Strategy

- **Temporary install root**: each invocation installs binaries into a temporary `--root` directory,
  executes the selected binary, then removes that directory on exit.
- **Persistent package cache**: build/intermediate artifacts are stored in the package cache directory
  and reused by future invocations.
- **Cargo global cache**: Cargo's own registry/git cache remains managed by Cargo (typically under
  `CARGO_HOME`, for example `~/.cargo`).

## Options

- `--cache-dir <dir>` sets the package build cache directory (`CARGO_TARGET_DIR` for install).
- `--clear` clears temporary install roots and package build cache paths derived from
  `CARGO_DLX_ROOT`/`CARGO_DLX_TEMP`/`CARGO_DLX_BUILD` (or the directory passed via `--cache-dir`).
- `CARGO_DLX_ROOT` sets the cargo-dlx runtime root directory (defaults to `~/.cargo-dlx`).
- `CARGO_DLX_TEMP` sets the temporary install base directory (defaults to `$CARGO_DLX_ROOT/tmp`).
- `CARGO_DLX_BUILD` sets the build cache base directory (defaults to `$CARGO_DLX_ROOT/build`,
  with `target` as its Cargo target subdirectory).
- Rust feature and lock/network flags are forwarded to `cargo install`:
  - `-F`, `--features`
  - `--all-features`
  - `--no-default-features`
  - `--locked`
  - `--offline`
  - `--frozen`

## Contribute

See [`CONTRIBUTE.md`](CONTRIBUTE.md) for more information on how to contribute to this project.

## License

This project is licensed under the [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) license.
