use std::{ffi::OsString, path::PathBuf, str::FromStr};

use clap::ArgAction;
use semver::{Version, VersionReq};

const CACHE_HEADING: &str = "Cache";
const RUST_OPTIONS_HEADING: &str = "Rust Options";
const SUBCOMMAND_NAME: &str = "dlx";

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "cargo dlx")]
#[command(bin_name = "cargo dlx")]
#[command(about = "A cargo subcommand for running remote binaries.")]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
#[command(disable_version_flag = true)]
#[command(disable_help_flag = true)]
#[command(
    after_help = "Visit https://github.com/Embers-of-the-Fire/cargo-dlx for documentation about this command."
)]
pub struct Cli {
    #[arg(
        value_name = "CRATE[@<VER>]",
        help = "Package to download, compile, and execute"
    )]
    pub krate: CrateSpec,

    #[arg(
        value_name = "ARGS",
        help = "Arguments passed to the downloaded binary",
        trailing_var_arg = true,
        allow_hyphen_values = true,
        num_args = 0..
    )]
    pub args: Vec<OsString>,

    #[arg(
        short = 'c',
        long = "shell-mode",
        help = "Run the command inside a shell"
    )]
    pub shell_mode: bool,

    #[arg(
        long,
        value_name = "DIR",
        help = "Directory used to cache package build artifacts",
        help_heading = CACHE_HEADING
    )]
    pub cache_dir: Option<PathBuf>,

    #[arg(
        long,
        help = "Disable package build artifact caching",
        help_heading = CACHE_HEADING,
        conflicts_with = "cache_dir"
    )]
    pub no_package_cache: bool,

    #[arg(
        short = 'F',
        long,
        value_name = "FEATURES",
        value_delimiter = ',',
        action = ArgAction::Append,
        help = "Space or comma separated list of features to activate",
        help_heading = RUST_OPTIONS_HEADING
    )]
    pub features: Vec<String>,

    #[arg(
        long,
        help = "Activate all available features",
        help_heading = RUST_OPTIONS_HEADING
    )]
    pub all_features: bool,

    #[arg(
        long,
        help = "Do not activate the `default` feature",
        help_heading = RUST_OPTIONS_HEADING
    )]
    pub no_default_features: bool,

    #[arg(
        long,
        help = "Assert that `Cargo.lock` will remain unchanged",
        help_heading = RUST_OPTIONS_HEADING
    )]
    pub locked: bool,

    #[arg(
        long,
        help = "Run without accessing the network",
        help_heading = RUST_OPTIONS_HEADING
    )]
    pub offline: bool,

    #[arg(
        long,
        help = "Equivalent to specifying both --locked and --offline",
        help_heading = RUST_OPTIONS_HEADING
    )]
    pub frozen: bool,
}

impl Cli {
    pub fn normalize_raw_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
        Self::normalize_raw_args_with_cargo_env(args, std::env::var_os("CARGO"))
    }

    fn normalize_raw_args_with_cargo_env(
        args: impl IntoIterator<Item = OsString>,
        cargo_env: Option<OsString>,
    ) -> Vec<OsString> {
        let mut normalized: Vec<OsString> = args.into_iter().collect();

        if cargo_env.is_some()
            && normalized
                .first()
                .and_then(|arg| arg.to_str())
                .is_some_and(|value| value == SUBCOMMAND_NAME)
        {
            normalized.remove(0);
        }

        normalized
    }

    pub fn wants_help(args: impl IntoIterator<Item = OsString>) -> bool {
        let mut expects_value = false;

        for arg in args {
            let Some(value) = arg.to_str() else {
                return false;
            };

            if expects_value {
                expects_value = false;
                continue;
            }

            if value == "--" {
                break;
            }

            if value == "-h" || value == "--help" {
                return true;
            }

            if Self::is_value_option_with_equals(value) || Self::is_short_value_option_inline(value)
            {
                continue;
            }

            if Self::is_value_option(value) {
                expects_value = true;
                continue;
            }

            if value.starts_with('-') {
                continue;
            }

            return false;
        }

        false
    }

    fn is_value_option(value: &str) -> bool {
        matches!(value, "--features" | "-F" | "--cache-dir")
    }

    fn is_value_option_with_equals(value: &str) -> bool {
        let Some((option, _)) = value.split_once('=') else {
            return false;
        };

        Self::is_value_option(option)
    }

    fn is_short_value_option_inline(value: &str) -> bool {
        value.starts_with("-F") && value.len() > 2
    }

    pub fn validate(&self) -> Result<(), clap::Error> {
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
        .ok_or_else(|| "no version provided for the package spec".to_owned())?;

    if let Some(stripped) = value.strip_prefix('v') {
        return Err(format!(
            "the version provided, `{value}` is not a valid SemVer requirement\n\nhelp: try changing the version to `{stripped}`"
        ));
    }

    let is_requirement = "<>=^~".contains(first) || value.contains('*');
    if is_requirement {
        return VersionReq::parse(value)
            .map_err(|_| format!("the version `{value}` is not a valid semver requirement"));
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

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use clap::{CommandFactory, Parser};
    use semver::VersionReq;

    use super::{Cli, CrateSpec};

    #[test]
    fn command_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_crate_and_forwarded_args() {
        let cli = Cli::parse_from([
            "cargo-dlx",
            "ripgrep@14.1.1",
            "--help",
            "--json",
            "--color",
            "always",
        ]);

        assert_eq!(
            cli.krate,
            CrateSpec {
                name: "ripgrep".to_owned(),
                version: Some(VersionReq::parse("=14.1.1").unwrap()),
            }
        );
        assert_eq!(cli.args, vec!["--help", "--json", "--color", "always"]);
    }

    #[test]
    fn parses_positional_crate_without_version() {
        let cli = Cli::parse_from(["cargo-dlx", "cargo-nextest"]);
        assert_eq!(cli.krate.name, "cargo-nextest");
        assert_eq!(cli.krate.version, None);
    }

    #[test]
    fn parses_positional_crate_with_attached_version() {
        let cli = Cli::parse_from(["cargo-dlx", "ripgrep@14.1.1", "--version"]);

        assert_eq!(
            cli.krate,
            CrateSpec {
                name: "ripgrep".to_owned(),
                version: Some(VersionReq::parse("=14.1.1").unwrap()),
            }
        );
        assert_eq!(cli.args, vec!["--version"]);
    }

    #[test]
    fn wants_help_before_crate() {
        assert!(Cli::wants_help(["--help".into()]));
        assert!(Cli::wants_help([
            "--cache-dir".into(),
            "/tmp/cargo-dlx-cache".into(),
            "--help".into(),
        ]));
        assert!(!Cli::wants_help(["rg".into(), "--help".into()]));
    }

    #[test]
    fn strips_subcommand_prefix_when_invoked_by_cargo() {
        let args = Cli::normalize_raw_args_with_cargo_env(
            ["dlx".into(), "ripgrep".into()],
            Some("cargo".into()),
        );

        assert_eq!(args, vec![OsString::from("ripgrep")]);
    }

    #[test]
    fn keeps_standalone_arguments_without_cargo_env() {
        let args = Cli::normalize_raw_args_with_cargo_env(["dlx".into(), "ripgrep".into()], None);

        assert_eq!(args, vec![OsString::from("dlx"), OsString::from("ripgrep")]);
    }

    #[test]
    fn detects_help_after_subcommand_normalization() {
        let args = Cli::normalize_raw_args_with_cargo_env(
            ["dlx".into(), "--help".into()],
            Some("cargo".into()),
        );

        assert!(Cli::wants_help(args));
    }

    #[test]
    fn parses_cache_options() {
        let cli = Cli::parse_from([
            "cargo-dlx",
            "--cache-dir",
            "/tmp/cargo-dlx-cache",
            "ripgrep",
        ]);

        assert_eq!(cli.cache_dir, Some(PathBuf::from("/tmp/cargo-dlx-cache")));
        assert!(!cli.no_package_cache);
    }
}
