use std::{ffi::OsString, path::PathBuf, str::FromStr};

use clap::ArgAction;
use semver::{Version, VersionReq};
use url::Url;

const FEATURE_HEADING: &str = "Feature Selection";
const COMPILATION_HEADING: &str = "Compilation Options";
const MANIFEST_HEADING: &str = "Manifest Options";
const SUBCOMMAND_NAME: &str = "dlx";

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "cargo dlx")]
#[command(bin_name = "cargo dlx")]
#[command(about = "A cargo subcommand for running remote binaries.")]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
#[command(disable_version_flag = true)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "Visit https://github.com/Embers-of-the-Fire/cargo-dlx for documentation about this command."
)]
pub struct Cli {
    #[arg(
        value_names = ["CRATE[@<VER>]", "ARG"],
        trailing_var_arg = true,
        allow_hyphen_values = true,
        num_args = 1..,
        help = "Package to download, compile, and execute"
    )]
    pub krate_and_args: Vec<OsString>,

    #[arg(
        long,
        value_name = "DIR",
        help = "Directory used to cache package build artifacts",
        help_heading = COMPILATION_HEADING
    )]
    pub cache_dir: Option<PathBuf>,

    #[arg(
        long,
        help = "Disable package build artifact caching",
        help_heading = COMPILATION_HEADING,
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
        help_heading = FEATURE_HEADING
    )]
    pub features: Vec<String>,

    #[arg(
        long,
        help = "Activate all available features",
        help_heading = FEATURE_HEADING
    )]
    pub all_features: bool,

    #[arg(
        long,
        help = "Do not activate the `default` feature",
        help_heading = FEATURE_HEADING
    )]
    pub no_default_features: bool,

    #[arg(
        long,
        help = "Assert that `Cargo.lock` will remain unchanged",
        help_heading = MANIFEST_HEADING
    )]
    pub locked: bool,

    #[arg(
        long,
        help = "Run without accessing the network",
        help_heading = MANIFEST_HEADING
    )]
    pub offline: bool,

    #[arg(
        long,
        help = "Equivalent to specifying both --locked and --offline",
        help_heading = MANIFEST_HEADING
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

    pub fn validate(&self) -> Result<(), clap::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSpec {
    pub package: Option<String>,
    pub version: Option<VersionReq>,
    pub source: PackageSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    CratesIo,
    RegistryIndex {
        index: String,
    },
    Git {
        url: String,
        reference: Option<GitReference>,
    },
    Path {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitReference {
    Branch(String),
    Tag(String),
    Rev(String),
}

impl FromStr for CrateSpec {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.contains("://") {
            return parse_source_crate_spec(value);
        }

        parse_registry_crate_spec(value)
    }
}

fn parse_registry_crate_spec(value: &str) -> Result<CrateSpec, String> {
    let (package, version) = if let Some((package, version)) = value.split_once('@') {
        if package.is_empty() {
            return Err("missing crate name before '@'".to_owned());
        }

        (Some(package.to_owned()), Some(parse_semver_flag(version)?))
    } else {
        (Some(value.to_owned()), None)
    };

    if package.as_ref().is_some_and(|package| package.is_empty()) {
        return Err("crate name is empty".to_owned());
    }

    Ok(CrateSpec {
        package,
        version,
        source: PackageSource::CratesIo,
    })
}

fn parse_source_crate_spec(value: &str) -> Result<CrateSpec, String> {
    let (kind, source_url) = split_source_kind(value)?;
    let mut url = Url::parse(&source_url)
        .map_err(|error| format!("invalid package source reference `{value}`: {error}"))?;

    let fragment = url.fragment().map(str::to_owned);
    url.set_fragment(None);
    let (fragment_package, fragment_version) = parse_source_fragment(fragment.as_deref())?;

    let source_kind = resolve_source_kind(kind.as_deref(), &url);
    let mut package = fragment_package;
    let version = fragment_version;

    if package.is_none()
        && version.is_some()
        && matches!(source_kind, SourceKind::Git | SourceKind::Path)
    {
        package = infer_package_name_from_url(&url);
    }

    if package.is_none() && matches!(source_kind, SourceKind::RegistryIndex) {
        return Err(format!(
            "registry package references must include a package name, like `{value}#my-crate`"
        ));
    }

    if package.is_none() && version.is_some() {
        return Err(format!(
            "package references with a version must include a package name, like `{value}#my-crate@1.0.0`"
        ));
    }

    let source = match source_kind {
        SourceKind::Git => {
            let reference = parse_git_reference(&url)?;
            url.set_query(None);

            PackageSource::Git {
                url: url.to_string(),
                reference,
            }
        }
        SourceKind::Path => {
            if url.query().is_some() {
                return Err(format!(
                    "local package references do not support URL query parameters: `{value}`"
                ));
            }

            let path = parse_local_file_path(value, &url)?;

            PackageSource::Path { path }
        }
        SourceKind::RegistryIndex => {
            if url.query().is_some_and(is_git_reference_query) {
                return Err(format!(
                    "registry package references do not support git query parameters: `{value}`"
                ));
            }

            let mut index = url.to_string();
            if kind.as_deref() == Some("sparse") {
                index = format!("sparse+{index}");
            }

            PackageSource::RegistryIndex { index }
        }
    };

    Ok(CrateSpec {
        package,
        version,
        source,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Git,
    Path,
    RegistryIndex,
}

fn split_source_kind(value: &str) -> Result<(Option<String>, String), String> {
    let Some(scheme_index) = value.find("://") else {
        return Ok((None, value.to_owned()));
    };

    let Some(kind_index) = value[..scheme_index].find('+') else {
        return Ok((None, value.to_owned()));
    };

    let kind = &value[..kind_index];
    let protocol = &value[kind_index + 1..scheme_index];

    if kind.is_empty() {
        return Err(format!("package source kind is missing in `{value}`"));
    }

    if protocol.is_empty() {
        return Err(format!("package source protocol is missing in `{value}`"));
    }

    if !matches!(kind, "git" | "path" | "registry" | "sparse" | "file") {
        return Err(format!(
            "unsupported package source kind `{kind}` (expected one of: git, path, file, registry, sparse)"
        ));
    }

    Ok((
        Some(kind.to_owned()),
        format!("{}://{}", protocol, &value[scheme_index + 3..]),
    ))
}

fn resolve_source_kind(kind: Option<&str>, url: &Url) -> SourceKind {
    match kind {
        Some("git") => SourceKind::Git,
        Some("path") | Some("file") => SourceKind::Path,
        Some("registry") | Some("sparse") => SourceKind::RegistryIndex,
        _ => infer_source_kind(url),
    }
}

fn infer_source_kind(url: &Url) -> SourceKind {
    if matches!(url.scheme(), "file") {
        return SourceKind::Path;
    }

    if matches!(url.scheme(), "ssh" | "git") {
        return SourceKind::Git;
    }

    if url.query().is_some_and(is_git_reference_query) {
        return SourceKind::Git;
    }

    if url.path().ends_with(".git") {
        return SourceKind::Git;
    }

    SourceKind::RegistryIndex
}

fn parse_source_fragment(
    fragment: Option<&str>,
) -> Result<(Option<String>, Option<VersionReq>), String> {
    let Some(fragment) = fragment else {
        return Ok((None, None));
    };

    if fragment.is_empty() {
        return Err("missing package fragment after '#'".to_owned());
    }

    if let Some((package, version)) = fragment.split_once('@') {
        if package.is_empty() {
            return Err("missing package name before '@'".to_owned());
        }

        return Ok((Some(package.to_owned()), Some(parse_semver_flag(version)?)));
    }

    if let Ok(version) = parse_semver_flag(fragment) {
        return Ok((None, Some(version)));
    }

    Ok((Some(fragment.to_owned()), None))
}

fn parse_git_reference(url: &Url) -> Result<Option<GitReference>, String> {
    let Some(query) = url.query() else {
        return Ok(None);
    };

    let mut reference = None;

    for (key, value) in url.query_pairs() {
        let next = match key.as_ref() {
            "branch" => GitReference::Branch(value.into_owned()),
            "tag" => GitReference::Tag(value.into_owned()),
            "rev" => GitReference::Rev(value.into_owned()),
            other => {
                return Err(format!(
                    "unsupported git query key `{other}` in package reference `{}`",
                    url.as_str()
                ));
            }
        };

        if reference.is_some() {
            return Err(format!(
                "git package references may only define one of `branch`, `tag`, or `rev`: `{}`",
                url.as_str()
            ));
        }

        reference = Some(next);
    }

    if query.is_empty() {
        return Err(format!(
            "git package reference query is empty in `{}`",
            url.as_str()
        ));
    }

    Ok(reference)
}

fn parse_local_file_path(value: &str, url: &Url) -> Result<PathBuf, String> {
    if let Ok(path) = url.to_file_path() {
        return Ok(path);
    }

    Err(format!(
        "package source `{value}` is not a valid local file URL"
    ))
}

fn is_git_reference_query(query: &str) -> bool {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('=').map(|(key, _)| key))
        .any(|key| matches!(key, "branch" | "tag" | "rev"))
}

fn infer_package_name_from_url(url: &Url) -> Option<String> {
    let segment = url.path_segments()?.rfind(|segment| !segment.is_empty())?;

    let inferred = segment.strip_suffix(".git").unwrap_or(segment);
    if inferred.is_empty() {
        return None;
    }

    Some(inferred.to_owned())
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

    use super::{Cli, CrateSpec, GitReference, PackageSource};

    #[test]
    fn command_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_crate_with_version() {
        let spec = "ripgrep@14.1.1".parse::<CrateSpec>().unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("ripgrep".to_owned()),
                version: Some(VersionReq::parse("=14.1.1").unwrap()),
                source: PackageSource::CratesIo,
            }
        );
    }

    #[test]
    fn parse_crate() {
        let spec = "ripgrep".parse::<CrateSpec>().unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("ripgrep".to_owned()),
                version: None,
                source: PackageSource::CratesIo,
            }
        );
    }

    #[test]
    fn parse_git_source_reference() {
        let spec = "git+https://example.com/tools/dlx-runner.git?rev=abc123#dlx-runner@0.3.0"
            .parse::<CrateSpec>()
            .unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("dlx-runner".to_owned()),
                version: Some(VersionReq::parse("=0.3.0").unwrap()),
                source: PackageSource::Git {
                    url: "https://example.com/tools/dlx-runner.git".to_owned(),
                    reference: Some(GitReference::Rev("abc123".to_owned())),
                },
            }
        );
    }

    #[test]
    fn parse_file_source_reference() {
        let path = std::env::temp_dir().join("dlx-file-ref");
        let url = url::Url::from_file_path(&path).unwrap();
        let spec = format!("{url}#dlx-file-ref").parse::<CrateSpec>().unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("dlx-file-ref".to_owned()),
                version: None,
                source: PackageSource::Path { path },
            }
        );
    }

    #[test]
    fn parse_file_kind_source_reference() {
        let path = std::env::temp_dir().join("dlx-file-kind-ref");
        let url = url::Url::from_file_path(&path).unwrap();
        let spec = format!("file+{url}#dlx-file-kind-ref")
            .parse::<CrateSpec>()
            .unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("dlx-file-kind-ref".to_owned()),
                version: None,
                source: PackageSource::Path { path },
            }
        );
    }

    #[test]
    fn parse_registry_source_reference() {
        let spec = "registry+https://github.com/rust-lang/crates.io-index#ripgrep@14.1.1"
            .parse::<CrateSpec>()
            .unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("ripgrep".to_owned()),
                version: Some(VersionReq::parse("=14.1.1").unwrap()),
                source: PackageSource::RegistryIndex {
                    index: "https://github.com/rust-lang/crates.io-index".to_owned(),
                },
            }
        );
    }

    #[test]
    fn parse_sparse_source_reference() {
        let spec = "sparse+https://index.crates.io/#cargo-edit"
            .parse::<CrateSpec>()
            .unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("cargo-edit".to_owned()),
                version: None,
                source: PackageSource::RegistryIndex {
                    index: "sparse+https://index.crates.io/".to_owned(),
                },
            }
        );
    }

    #[test]
    fn infers_package_name_for_git_version_fragment() {
        let spec = "git+https://example.com/tools/dlx-runner.git#1.2.3"
            .parse::<CrateSpec>()
            .unwrap();

        assert_eq!(
            spec,
            CrateSpec {
                package: Some("dlx-runner".to_owned()),
                version: Some(VersionReq::parse("=1.2.3").unwrap()),
                source: PackageSource::Git {
                    url: "https://example.com/tools/dlx-runner.git".to_owned(),
                    reference: None,
                },
            }
        );
    }

    #[test]
    fn rejects_registry_source_without_package_name() {
        let error = "registry+https://github.com/rust-lang/crates.io-index"
            .parse::<CrateSpec>()
            .unwrap_err();

        assert!(error.contains("must include a package name"));
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
            cli.krate_and_args,
            vec!["ripgrep@14.1.1", "--help", "--json", "--color", "always"]
        );
    }

    #[test]
    fn parses_positional_crate_without_version() {
        let cli = Cli::parse_from(["cargo-dlx", "cargo-nextest"]);
        assert_eq!(cli.krate_and_args, vec!["cargo-nextest"]);
    }

    #[test]
    fn parses_positional_crate_with_attached_version() {
        let cli = Cli::parse_from(["cargo-dlx", "ripgrep@14.1.1", "--version"]);
        assert_eq!(cli.krate_and_args, vec!["ripgrep@14.1.1", "--version"]);
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
