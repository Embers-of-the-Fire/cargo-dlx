# `cargo dlx`

`cargo dlx` downloads, compiles, and executes a Rust binary without installing it globally.

This package is in its early stages of development, and the API may change without a major version bump.

For design of the command, please refer to [DESIGN.md](DESIGN.md).
Feel free to discuss about design by creating a Pull Request against the document.

## Usage

```bash
cargo dlx ripgrep@14.1.1 --help
```

The command installs into a temporary directory, runs the binary, then removes the temporary files.
`cargo-dlx` invokes the Cargo executable from `$CARGO` when set, otherwise `cargo` from `PATH`.
Package build artifacts are cached in a persistent Cargo target directory (default under cache home)
to speed up repeated runs, while installed binaries stay temporary.

## Cache Strategy

`cargo-dlx` caches **package build artifacts** only (via `CARGO_TARGET_DIR`) to speed up repeated runs.
It does **not** cache installed runnable binaries between invocations.

Cache directory selection (highest priority first):

1. `--cache-dir <DIR>`
2. `CARGO_DLX_CACHE_DIR`
3. Platform default cache path

You can disable package caching with `--no-package-cache`.

## Storage Strategy

- **Ephemeral install root**: each invocation installs binaries into a temporary `--root` directory,
  executes the selected binary, then removes that directory on exit.
- **Persistent package cache**: build/intermediate artifacts are stored in the package cache directory
  (when enabled) and reused by future invocations.
- **Cargo global cache**: Cargo's own registry/git cache remains managed by Cargo (typically under
  `CARGO_HOME`, for example `~/.cargo`).

Default package cache locations:

- Unix: `$XDG_CACHE_HOME/cargo-dlx/target` or fallback `$HOME/.cache/cargo-dlx/target`
- Windows: `%LOCALAPPDATA%\\cargo-dlx\\target` or fallback `%USERPROFILE%\\AppData\\Local\\cargo-dlx\\target`

## Options

- `-c`, `--shell-mode` executes the installed binary through your shell.
- `--cache-dir <dir>` sets the package build cache directory (`CARGO_TARGET_DIR` for install).
- `--no-package-cache` disables package cache usage.
- `CARGO_DLX_CACHE_DIR` sets the package cache directory when `--cache-dir` is not used.
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
