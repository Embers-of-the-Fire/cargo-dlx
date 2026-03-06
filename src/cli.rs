use std::{ffi::OsString, path::PathBuf, str::FromStr};

use clap::{ArgAction, ValueEnum, ValueHint};
use semver::{Version, VersionReq};

const MANIFEST_OPTIONS_HEADING: &str = "Manifest Options";
const TARGET_SELECTION_HEADING: &str = "Target Selection";
const FEATURE_SELECTION_HEADING: &str = "Feature Selection";
const COMPILATION_OPTIONS_HEADING: &str = "Compilation Options";

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "cargo dlx")]
#[command(bin_name = "cargo dlx")]
#[command(about = "Run a Rust binary")]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
#[command(disable_version_flag = true)]
#[command(after_help = "Run `cargo help dlx` for more detailed information.")]
pub struct Cli {
    #[arg(
        id = "crate",
        value_name = "CRATE[@<VER>]",
        help = "Select the package from the given source",
        num_args = 0..
    )]
    pub crates: Vec<CrateSpec>,

    #[arg(
        long,
        value_name = "VERSION",
        help = "Specify a version to install",
        value_parser = parse_semver_flag,
        requires = "crate"
    )]
    pub version: Option<VersionReq>,

    #[arg(
        long,
        value_name = "INDEX",
        help = "Registry index to install from",
        requires = "crate",
        conflicts_with_all = ["git", "path", "registry"]
    )]
    pub index: Option<String>,

    #[arg(
        long,
        value_name = "REGISTRY",
        help = "Registry to use",
        requires = "crate",
        conflicts_with_all = ["git", "path", "index"]
    )]
    pub registry: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Git URL to install the specified crate from",
        conflicts_with_all = ["path", "index", "registry"]
    )]
    pub git: Option<String>,

    #[arg(
        long,
        value_name = "BRANCH",
        help = "Branch to use when installing from git",
        requires = "git"
    )]
    pub branch: Option<String>,

    #[arg(
        long,
        value_name = "TAG",
        help = "Tag to use when installing from git",
        requires = "git"
    )]
    pub tag: Option<String>,

    #[arg(
        long,
        value_name = "SHA",
        help = "Specific commit to use when installing from git",
        requires = "git"
    )]
    pub rev: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Filesystem path to local crate to install from",
        value_hint = ValueHint::AnyPath,
        conflicts_with_all = ["git", "index", "registry"]
    )]
    pub path: Option<PathBuf>,

    #[arg(
        long,
        value_name = "DIR",
        help = "Directory to install packages into",
        value_hint = ValueHint::DirPath
    )]
    pub root: Option<PathBuf>,

    #[arg(
        short = 'f',
        long,
        help = "Force overwriting existing crates or binaries"
    )]
    pub force: bool,

    #[arg(
        short = 'n',
        long,
        help = "Perform all checks without installing (unstable)"
    )]
    pub dry_run: bool,

    #[arg(long, help = "Do not save tracking information")]
    pub no_track: bool,

    #[arg(long, help = "List all installed packages and their versions")]
    pub list: bool,

    #[arg(long, value_name = "FMT", help = "Error format")]
    pub message_format: Option<MessageFormat>,

    #[arg(
        long,
        help = "Build in debug mode (with the 'dev' profile) instead of release mode",
        conflicts_with = "profile"
    )]
    pub debug: bool,

    #[arg(
        short = 'v',
        long,
        action = ArgAction::Count,
        help = "Use verbose output (-vv very verbose/build.rs output)"
    )]
    pub verbose: u8,

    #[arg(short = 'q', long, help = "Do not print cargo log messages")]
    pub quiet: bool,

    #[arg(long, value_name = "WHEN", help = "Coloring")]
    pub color: Option<ColorChoice>,

    #[arg(
        long,
        value_name = "KEY=VALUE|PATH",
        help = "Override a configuration value",
        action = ArgAction::Append
    )]
    pub config: Vec<String>,

    #[arg(
        short = 'Z',
        value_name = "FLAG",
        help = "Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details",
        action = ArgAction::Append
    )]
    pub unstable_flags: Vec<String>,

    #[arg(
        long,
        help = "Ignore `rust-version` specification in packages",
        help_heading = MANIFEST_OPTIONS_HEADING
    )]
    pub ignore_rust_version: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Path to Cargo.lock (unstable)",
        value_hint = ValueHint::FilePath,
        help_heading = MANIFEST_OPTIONS_HEADING
    )]
    pub lockfile_path: Option<PathBuf>,

    #[arg(
        long,
        help = "Assert that `Cargo.lock` will remain unchanged",
        help_heading = MANIFEST_OPTIONS_HEADING
    )]
    pub locked: bool,

    #[arg(
        long,
        help = "Run without accessing the network",
        help_heading = MANIFEST_OPTIONS_HEADING
    )]
    pub offline: bool,

    #[arg(
        long,
        help = "Equivalent to specifying both --locked and --offline",
        help_heading = MANIFEST_OPTIONS_HEADING
    )]
    pub frozen: bool,

    #[arg(
        long,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "",
        action = ArgAction::Append,
        help = "Install only the specified binary",
        help_heading = TARGET_SELECTION_HEADING
    )]
    pub bin: Vec<String>,

    #[arg(
        long,
        help = "Install all binaries",
        help_heading = TARGET_SELECTION_HEADING
    )]
    pub bins: bool,

    #[arg(
        long,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "",
        action = ArgAction::Append,
        help = "Install only the specified example",
        help_heading = TARGET_SELECTION_HEADING
    )]
    pub example: Vec<String>,

    #[arg(
        long,
        help = "Install all examples",
        help_heading = TARGET_SELECTION_HEADING
    )]
    pub examples: bool,

    #[arg(
        short = 'F',
        long,
        value_name = "FEATURES",
        value_delimiter = ',',
        action = ArgAction::Append,
        help = "Space or comma separated list of features to activate",
        help_heading = FEATURE_SELECTION_HEADING
    )]
    pub features: Vec<String>,

    #[arg(
        long,
        help = "Activate all available features",
        help_heading = FEATURE_SELECTION_HEADING
    )]
    pub all_features: bool,

    #[arg(
        long,
        help = "Do not activate the `default` feature",
        help_heading = FEATURE_SELECTION_HEADING
    )]
    pub no_default_features: bool,

    #[arg(
        short = 'j',
        long,
        value_name = "N",
        help = "Number of parallel jobs, defaults to # of CPUs.",
        help_heading = COMPILATION_OPTIONS_HEADING
    )]
    pub jobs: Option<u32>,

    #[arg(
        long,
        help = "Do not abort the build as soon as there is an error",
        help_heading = COMPILATION_OPTIONS_HEADING
    )]
    pub keep_going: bool,

    #[arg(
        long,
        value_name = "PROFILE-NAME",
        help = "Install artifacts with the specified profile",
        help_heading = COMPILATION_OPTIONS_HEADING
    )]
    pub profile: Option<String>,

    #[arg(
        long,
        value_name = "TRIPLE",
        num_args = 0..=1,
        default_missing_value = "",
        action = ArgAction::Append,
        help = "Build for the target triple",
        help_heading = COMPILATION_OPTIONS_HEADING
    )]
    pub target: Vec<String>,

    #[arg(
        long,
        value_name = "DIRECTORY",
        help = "Directory for all generated artifacts",
        value_hint = ValueHint::DirPath,
        help_heading = COMPILATION_OPTIONS_HEADING
    )]
    pub target_dir: Option<PathBuf>,

    #[arg(
        long,
        value_name = "FMTS",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "html",
        value_delimiter = ',',
        help = "Timing output formats (unstable) (comma separated): html, json",
        value_enum,
        help_heading = COMPILATION_OPTIONS_HEADING
    )]
    pub timings: Option<Vec<TimingFormat>>,

    #[arg(
        value_name = "ARGS",
        help = "Arguments to pass to the executed binary",
        allow_hyphen_values = true,
        last = true,
        num_args = 0..
    )]
    pub args: Vec<OsString>,
}

impl Cli {
    pub fn validate(&self) -> Result<(), clap::Error> {
        if self.version.is_some() && self.crates.iter().any(|spec| spec.version.is_some()) {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::ArgumentConflict,
                "cannot specify both `@<VERSION>` and `--version <VERSION>`",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSpec {
    pub name: String,
    pub version: Option<VersionReq>,
}

impl FromStr for CrateSpec {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (name, version) = if let Some((name, version)) = value.split_once('@') {
            if name.is_empty() {
                return Err("missing crate name before '@'".to_owned());
            }

            (name.to_owned(), Some(parse_semver_flag(version)?))
        } else {
            (value.to_owned(), None)
        };

        if name.is_empty() {
            return Err("crate name is empty".to_owned());
        }

        Ok(Self { name, version })
    }
}

fn parse_semver_flag(value: &str) -> Result<VersionReq, String> {
    let first = value
        .chars()
        .next()
        .ok_or_else(|| "no version provided for the `--version` flag".to_owned())?;

    if let Some(stripped) = value.strip_prefix('v') {
        return Err(format!(
            "the version provided, `{value}` is not a valid SemVer requirement\n\nhelp: try changing the version to `{stripped}`"
        ));
    }

    let is_requirement = "<>=^~".contains(first) || value.contains('*');
    if is_requirement {
        return VersionReq::parse(value).map_err(|_| {
            format!(
                "the `--version` provided, `{value}`, is not a valid semver version requirement"
            )
        });
    }

    match value.trim().parse::<Version>() {
        Ok(version) => VersionReq::parse(&format!("={version}")).map_err(|error| error.to_string()),
        Err(error) => {
            let mut message = error.to_string();

            if VersionReq::parse(value).is_ok() {
                message.push_str(&format!(
                    "\n\n  tip: if you want to specify SemVer range, add an explicit qualifier, like '^{value}'"
                ));
            }

            Err(message)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum MessageFormat {
    Human,
    Short,
    Json,
    JsonDiagnosticShort,
    JsonDiagnosticRenderedAnsi,
    JsonRenderDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum TimingFormat {
    Html,
    Json,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};
    use semver::VersionReq;

    use super::{Cli, CrateSpec, TimingFormat, parse_semver_flag};

    #[test]
    fn command_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_crate_with_attached_version() {
        let crate_spec: CrateSpec = "ripgrep@14.1.1".parse().unwrap();
        assert_eq!(crate_spec.name, "ripgrep");
        assert_eq!(
            crate_spec.version,
            Some(VersionReq::parse("=14.1.1").unwrap())
        );
    }

    #[test]
    fn parses_exact_version_as_requirement() {
        let version = parse_semver_flag("1.2.3").unwrap();
        assert_eq!(version, VersionReq::parse("=1.2.3").unwrap());
    }

    #[test]
    fn parses_optional_multi_flags_and_timings() {
        let cli = Cli::parse_from([
            "cargo-dlx",
            "ripgrep",
            "--bin",
            "--example",
            "rg-example",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--timings",
            "--",
            "--help",
        ]);

        assert_eq!(cli.bin, vec![String::new()]);
        assert_eq!(cli.example, vec!["rg-example".to_owned()]);
        assert_eq!(cli.target, vec!["x86_64-unknown-linux-gnu".to_owned()]);
        assert_eq!(cli.timings, Some(vec![TimingFormat::Html]));
        assert_eq!(cli.args, vec!["--help"]);
    }

    #[test]
    fn rejects_conflicting_version_sources() {
        let cli = Cli::parse_from(["cargo-dlx", "ripgrep@14.1.1", "--version", "14.1.1"]);
        assert!(cli.validate().is_err());
    }
}
