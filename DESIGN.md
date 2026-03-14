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

### Decisions

### Specifying the package to run

To allow specifying packages from any dependency source,
`cargo dlx` accepts Cargo's
[Package Id Specifications](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html) format.

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

Alternatives:
- Only accept `<name>[@<ver>]` from the registry
- Have `<name>` pull from a local `Cargo.lock` like `cargo info`
- Allow `-` for running from stdin (or should that just be `cargo -`)
- Provide a `<ver>` identifier for using a local `Cargo.lock` for the version
- Have a bare `<name>` be a placeholder for the last source and combination of flags (bin, profile, features) that used that name

### Forwarding arguments

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

### Shell execution mode (`-c`, `--cmdline`)

`cargo dlx` also supports an `npx`-style command-line mode:

```console
$ cargo dlx -c '<SHELL_COMMAND>' -p <PACKAGE> [-p <PACKAGE> ...] --shell <SHELL>
$ cargo dlx --cmdline '<SHELL_COMMAND>' --package <PACKAGE> [--package <PACKAGE> ...] --shell <SHELL>
```

When shell execution mode is enabled:

- Positional package syntax is disabled. For example, `cargo dlx foo@1.2.3 -c 'foo --help'` is invalid.
- Packages are declared explicitly with repeatable `-p` / `--package`.
- All specified packages are installed into the same temporary install directory.
- Installed binaries are injected into the shell execution environment so `<SHELL_COMMAND>` can invoke them directly.
- `--shell` selects the shell executable used to run the command line.

In this mode, package arguments are part of the shell command string itself rather than passed as structured argv after package parsing.

### Multi-binary packages

While most packages have just one binary,
that isn't an inherent requirement.
In addition, users may wish to run an example.

`cargo dlx` will use the Cargo standard `--bin` and `--example` arguments to specify a specific binary to build and run.

Cargo semantics:
- a single `[[bin]]` is considered the default
- if multiple `[[bin]]`s are present, `package.default-run` can specify the default
- if there are multiple `[[bin]]`s without a default, error and list all `[[bin]]`s
- `--bin` or `--example` without a name lists available names

Implementation status:
- ✅ `--bin`
- ✅ `--example`
- ✅ error when ambiguous
- ❌ `package.default-run`

Alternatives:
- Have a syntax to mix this in with the package selection
- If a [`last`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.last) argument is present, the usage becomes `cargo dlx [DLX_ARGS] <PACKAGE> <BIN> -- [PACKAGE_ARGS]`
- Add a `--package` selection flag which changes `<PACKAGE>` to be a `<CMD>` run from that package

### Caching strategy

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

Note: overall, the cache strategy is opaque to the user and we can change how it is implemented over time.

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
- Add a way to disable long lived caching, always using emphemeral caches
  - If we did this, it should be driven by a user request and live in `.cargo/config.toml` (which also gets env variables and CLI support for free)

### Garbage collection strategy

Current behavior:

- `cargo dlx` keeps a runtime root at `CARGO_DLX_ROOT` (default `~/.cargo-dlx`).
- Temporary installation roots are created under `tmp/<timestamp>` and are removed automatically when the process exits.
- Build artifacts are cached under `build/target` and reused across invocations.
- `--clear` removes temporary install roots and package cache directories.

#### `--clear` Logic

`--clear` resolves directories independently and does not require a root when explicit temp/build paths are available.

1. Resolve `temp_base` in this order:
   - `CARGO_DLX_TEMP`
   - `<root>/tmp` where `<root>` is from `CARGO_DLX_ROOT` or `~/.cargo-dlx`
   - otherwise: error (`could not determine cargo-dlx temporary directory`)

2. Resolve `build_target` in this order:
   - `--cache-dir <DIR>` (used directly)
   - `CARGO_DLX_BUILD/target`
   - `<root>/build/target` where `<root>` is from `CARGO_DLX_ROOT` or `~/.cargo-dlx`
   - otherwise: error (`could not determine cargo-dlx build cache directory`)

3. Path normalization rules:
   - for env-based paths (`CARGO_DLX_TEMP`, `CARGO_DLX_BUILD`, `CARGO_DLX_ROOT`), absolute paths are used as-is, and relative paths are resolved against current working directory.
   - `--cache-dir` is used as provided.

4. Deletion steps:
   - remove `temp_base` recursively; ignore `NotFound`
   - if `build_target == temp_base`, stop (avoid duplicate delete)
   - otherwise remove `build_target` recursively; ignore `NotFound`

This means environments with no `HOME` and no `CARGO_DLX_ROOT` still support `cargo dlx --clear` when both temporary and build locations are explicit (via `CARGO_DLX_TEMP` + `CARGO_DLX_BUILD`, or `CARGO_DLX_TEMP` + `--cache-dir`).

## Profile

`cargo dlx` defaults to the `release` profile, and is configurable via:

- ✅ `--profile`

Whether the default can be overriden in a config file is dependent on feedback,
including gathering use cases for it.

### Package-Specific Configuration

`cargo dlx` would respect to user-defined package-specific configuration, but would not track them implicitly.
That is to say, `cargo dlx` would accept configuration written to its configuration files,
but would not automatically save them after a random call.

Implement status: Not implemented now.

## Prior art

### Design comparisons

#### [`yarn dlx`](https://yarnpkg.com/cli/dlx)

Usage: `yarn dlx [-p <name>]... <command> [arg]...`

In the current working directory, run the binary script from a package installed to a temporary environment.

Notes:
- Does not support specifying versions

Further investigation:
- Can a binary script have a different name than the package?  If so, then instead of `cargo dlx --bin cargo-remove cargo-edit`, the yarn equivalent is `cargo dlx -p cargo-edit cargo-remove`

#### [`pnpm dlx`](https://pnpm.io/cli/dlx)

Usage: `pnpm dlx [--allow-build] [--shell-mode] [--package <name>[@<ver>] <name>[@<ver>] [arg]...`

In the current working directory, run the binary script from a package installed to a temporary environment.

Notes:
- Supports specifying versions
  - The [`catalog:` protocol](https://pnpm.io/catalogs) allows for pulling versions from their equivalent of `workspace.dependencies`
- `--allow-build` is an allowlist for post-install scripts
- `--shell-mode` runs the command through a shell

#### [`npm exec`](https://docs.npmjs.com/cli/v11/commands/npm-exec/)

Usage:
- `npm exec -- <name>[@<ver>] [arg]...`
- `npm exec --package=<name>[@<ver>]... -- <cmd> [arg]...`
- `npm exec [--package=<name>[@<ver>]]... -c "<cmd> [arg]..."`

In the current working directory, run the binary script from a package installed to the current environment.

Notes:
- Versions default to what is already installed in the current environment
- Binary scripts from `--package` are put in `PATH`
- `--package` and `<cmd>` are used to select a specific binary in a package

#### [`npx`](https://docs.npmjs.com/cli/v11/commands/npx)

Like `npm exec` but no `npx` flags are allowed after the first positional argument.
([source](https://docs.npmjs.com/cli/v11/commands/npx#npx-vs-npm-exec)).

#### [`bunx`](https://bun.com/docs/pm/bunx)

aka `bun x`

Usage: `bunx [--bun] [--package <name>[@<ver>]]` <cmd> [arg]...`

bunx will check for a locally installed package first, then fall back to auto-installing the package from npm.
Installed packages will be stored in Bun’s global cache for future use.

Notes:
- Executes `<cmd>` using its shebang, `--bun` overrides that
- `--package` and `<cmd>` are used to select a specific binary in a package

#### [`pipx run`](https://pipx.pypa.io/stable/)

Usage: `pipx run [--spec <name>] -- [cmd] [arg]...`

Notes:
- `--spec` and `<cmd>` are used to select a specific binary in a package but then `.py` is needed on `[cmd]`
- `<name>` can be a package name, a package name and version requirement, a git URL, or a python file at a URL

#### [`uvx`](https://docs.astral.sh/uv/guides/tools/)

aka `uv tool run`

Usage: `uvx [--from <name>[@<ver-req>] [--with <name>[@<ver-req>] <cmd[@<ver>|@<latest>]> [arg]...`

Notes:
- `--from` can be a package name, a package name and version requirement, or a git URL
- `--with` is for specifying plugins

#### [`deno run`](https://docs.deno.com/runtime/reference/cli/run/)

Usage: `deno [--allow-scripts] run <source>`

Notes:
- `source` can be
  - a website hosting a source file
  - a registry identifier
  - `-` for reading from stdin (e.g. piping from `curl`)
- `--allow-scripts`: allow list for lifecycle scripts
- Includes flags for cache management (e.g. `--locked`, `--frozen`, `--cached-only`)
- Includes permission flags (e.g. `--allow-read`, `--allow-net`)
- `--watch` mode will kill and restart the process on `<source>` change
