use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    time::{SystemTime, UNIX_EPOCH},
};

use super::cli::{Cli, CrateSpec, GitReference, PackageSource};

const CARGO_DLX_ROOT_ENV: &str = "CARGO_DLX_ROOT";
const CARGO_DLX_TEMP_ENV: &str = "CARGO_DLX_TEMP";
const CARGO_DLX_BUILD_ENV: &str = "CARGO_DLX_BUILD";

const CARGO_DLX_ROOT_DIRNAME: &str = ".cargo-dlx";
const CARGO_DLX_TEMP_DIRNAME: &str = "tmp";
const CARGO_DLX_BUILD_DIRNAME: &str = "build";
const CARGO_DLX_TARGET_DIRNAME: &str = "target";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    Completed,
    ChildExited(i32),
}

#[derive(Debug, Clone)]
pub struct RunError {
    message: String,
    exit_code: i32,
}

impl RunError {
    fn new(message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            message: message.into(),
            exit_code,
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RunError {}

pub fn execute(cli: &Cli) -> Result<Execution, RunError> {
    if cli.clear {
        clear_cached_data(cli)?;
        return Ok(Execution::Completed);
    }

    let install_root = TempInstallRoot::new().map_err(|error| {
        RunError::new(
            format!("failed to create temporary install root: {error}"),
            1,
        )
    })?;

    let krate = cli.krate_and_args[0]
        .to_str()
        .ok_or_else(|| RunError::new("crate name is not valid a UTF-8 string".to_string(), 1))?;
    let krate = krate
        .parse::<CrateSpec>()
        .map_err(|err| RunError::new(err.to_string(), 1))?;
    let args = &cli.krate_and_args[1..];

    let install_status = install_package(&krate, cli, install_root.path())
        .map_err(|error| RunError::new(format!("failed to launch `cargo install`: {error}"), 1))?;

    if !install_status.success() {
        return Ok(Execution::ChildExited(exit_code_from_status(
            &install_status,
        )));
    }

    let executable = resolve_executable(&install_root.bin_dir(), krate.package.as_deref())?;

    let run_status = run_direct(&executable, args).map_err(|error| {
        RunError::new(
            format!("failed to execute `{}`: {error}", executable.display()),
            1,
        )
    })?;

    if run_status.success() {
        Ok(Execution::Completed)
    } else {
        Ok(Execution::ChildExited(exit_code_from_status(&run_status)))
    }
}

fn clear_cached_data(cli: &Cli) -> Result<(), RunError> {
    let directories = resolve_clear_directories(cli).map_err(|error| {
        RunError::new(
            format!("failed to resolve cargo-dlx cache directories: {error}"),
            1,
        )
    })?;

    remove_directory_if_exists(&directories.temp_base).map_err(|error| {
        RunError::new(
            format!(
                "failed to clear temporary install roots at `{}`: {error}",
                directories.temp_base.display()
            ),
            1,
        )
    })?;

    if directories.build_target == directories.temp_base {
        return Ok(());
    }

    remove_directory_if_exists(&directories.build_target).map_err(|error| {
        RunError::new(
            format!(
                "failed to clear package build cache at `{}`: {error}",
                directories.build_target.display()
            ),
            1,
        )
    })?;

    Ok(())
}

fn remove_directory_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClearDirectories {
    temp_base: PathBuf,
    build_target: PathBuf,
}

fn resolve_clear_directories(cli: &Cli) -> io::Result<ClearDirectories> {
    let cwd = std::env::current_dir()?;

    resolve_clear_directories_with(
        &cwd,
        cli.cache_dir.clone(),
        non_empty_env_os(CARGO_DLX_ROOT_ENV),
        non_empty_env_os(CARGO_DLX_TEMP_ENV),
        non_empty_env_os(CARGO_DLX_BUILD_ENV),
        default_home_dir(),
    )
}

fn resolve_clear_directories_with(
    cwd: &Path,
    cache_dir: Option<PathBuf>,
    root_env: Option<OsString>,
    temp_env: Option<OsString>,
    build_env: Option<OsString>,
    home_dir: Option<PathBuf>,
) -> io::Result<ClearDirectories> {
    let root = resolve_env_path(cwd, root_env).or_else(|| default_dlx_root_dir(home_dir));

    let temp_base = resolve_env_path(cwd, temp_env)
        .or_else(|| root.as_ref().map(|root| root.join(CARGO_DLX_TEMP_DIRNAME)))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "could not determine cargo-dlx temporary directory; set {CARGO_DLX_TEMP_ENV} or {CARGO_DLX_ROOT_ENV}"
                ),
            )
        })?;

    let build_target = if let Some(path) = cache_dir {
        path
    } else {
        resolve_env_path(cwd, build_env)
            .or_else(|| root.as_ref().map(|root| root.join(CARGO_DLX_BUILD_DIRNAME)))
            .map(|build_base| build_base.join(CARGO_DLX_TARGET_DIRNAME))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "could not determine cargo-dlx build cache directory; set --cache-dir, {CARGO_DLX_BUILD_ENV}, or {CARGO_DLX_ROOT_ENV}"
                    ),
                )
            })?
    };

    Ok(ClearDirectories {
        temp_base,
        build_target,
    })
}

fn install_package(krate: &CrateSpec, cli: &Cli, root: &Path) -> io::Result<ExitStatus> {
    let mut command = Command::new(cargo_binary());
    command.arg("install");

    match &krate.source {
        PackageSource::CratesIo => {
            if let Some(package) = &krate.package {
                command.arg(package);
            }
        }
        PackageSource::RegistryIndex { index } => {
            command.arg("--index");
            command.arg(index);

            if let Some(package) = &krate.package {
                command.arg(package);
            }
        }
        PackageSource::Git { url, reference } => {
            command.arg("--git");
            command.arg(url);

            if let Some(reference) = reference {
                match reference {
                    GitReference::Branch(branch) => {
                        command.arg("--branch");
                        command.arg(branch);
                    }
                    GitReference::Tag(tag) => {
                        command.arg("--tag");
                        command.arg(tag);
                    }
                    GitReference::Rev(rev) => {
                        command.arg("--rev");
                        command.arg(rev);
                    }
                }
            }

            if let Some(package) = &krate.package {
                command.arg(package);
            }
        }
        PackageSource::Path { path } => {
            command.arg("--path");
            command.arg(path);

            if let Some(package) = &krate.package {
                command.arg(package);
            }
        }
    }

    command.arg("--root");
    command.arg(root);

    command.arg("--profile");
    command.arg(&cli.profile);

    if let Some(version_req) = &krate.version {
        command.arg("--version");
        command.arg(version_req.to_string());
    }

    if !cli.features.is_empty() {
        command.arg("--features");
        command.arg(cli.features.join(","));
    }

    if cli.all_features {
        command.arg("--all-features");
    }

    if cli.no_default_features {
        command.arg("--no-default-features");
    }

    if cli.frozen {
        command.arg("--frozen");
    } else {
        if cli.locked {
            command.arg("--locked");
        }

        if cli.offline {
            command.arg("--offline");
        }
    }

    configure_package_cache(&mut command, cli)?;

    command.status()
}

fn cargo_binary() -> OsString {
    non_empty_env_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

fn configure_package_cache(command: &mut Command, cli: &Cli) -> io::Result<()> {
    let Some(cache_dir) = package_cache_dir(cli)? else {
        return Ok(());
    };

    fs::create_dir_all(&cache_dir)?;
    command.env("CARGO_TARGET_DIR", cache_dir);

    Ok(())
}

fn package_cache_dir(cli: &Cli) -> io::Result<Option<PathBuf>> {
    if cli.no_package_cache {
        return Ok(None);
    }

    if let Some(path) = &cli.cache_dir {
        return Ok(Some(path.clone()));
    }

    Ok(Some(resolve_dlx_directories()?.build_target_dir()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DlxDirectories {
    root: PathBuf,
    temp_base: PathBuf,
    build_base: PathBuf,
}

impl DlxDirectories {
    fn build_target_dir(&self) -> PathBuf {
        self.build_base.join(CARGO_DLX_TARGET_DIRNAME)
    }

    fn temp_base_dir(&self) -> &Path {
        &self.temp_base
    }
}

fn resolve_dlx_directories() -> io::Result<DlxDirectories> {
    let cwd = std::env::current_dir()?;

    resolve_dlx_directories_with(
        &cwd,
        non_empty_env_os(CARGO_DLX_ROOT_ENV),
        non_empty_env_os(CARGO_DLX_TEMP_ENV),
        non_empty_env_os(CARGO_DLX_BUILD_ENV),
        default_home_dir(),
    )
}

fn resolve_dlx_directories_with(
    cwd: &Path,
    root_env: Option<OsString>,
    temp_env: Option<OsString>,
    build_env: Option<OsString>,
    home_dir: Option<PathBuf>,
) -> io::Result<DlxDirectories> {
    let root = resolve_env_path(cwd, root_env)
        .or_else(|| default_dlx_root_dir(home_dir))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("could not determine cargo-dlx root directory; set {CARGO_DLX_ROOT_ENV}"),
            )
        })?;

    let temp_base =
        resolve_env_path(cwd, temp_env).unwrap_or_else(|| root.join(CARGO_DLX_TEMP_DIRNAME));
    let build_base =
        resolve_env_path(cwd, build_env).unwrap_or_else(|| root.join(CARGO_DLX_BUILD_DIRNAME));

    Ok(DlxDirectories {
        root,
        temp_base,
        build_base,
    })
}

fn resolve_env_path(cwd: &Path, env_value: Option<OsString>) -> Option<PathBuf> {
    let path = env_value.map(PathBuf::from)?;

    if path.is_absolute() {
        Some(path)
    } else {
        Some(cwd.join(path))
    }
}

fn default_dlx_root_dir(home_dir: Option<PathBuf>) -> Option<PathBuf> {
    home_dir.map(|home| home.join(CARGO_DLX_ROOT_DIRNAME))
}

#[cfg(windows)]
fn default_home_dir() -> Option<PathBuf> {
    if let Some(user_profile) = non_empty_env_os("USERPROFILE") {
        return Some(PathBuf::from(user_profile));
    }

    match (non_empty_env_os("HOMEDRIVE"), non_empty_env_os("HOMEPATH")) {
        (Some(home_drive), Some(home_path)) => {
            let mut home = PathBuf::from(home_drive);
            home.push(home_path);
            Some(home)
        }
        _ => None,
    }
}

#[cfg(not(windows))]
fn default_home_dir() -> Option<PathBuf> {
    non_empty_env_os("HOME").map(PathBuf::from)
}

fn non_empty_env_os(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

fn run_direct(executable: &Path, args: &[OsString]) -> io::Result<ExitStatus> {
    let mut command = Command::new(executable);
    command.args(args);

    command.status()
}

fn resolve_executable(bin_dir: &Path, package_name: Option<&str>) -> Result<PathBuf, RunError> {
    let mut entries = Vec::new();

    let read_dir = fs::read_dir(bin_dir).map_err(|error| {
        RunError::new(
            format!(
                "failed to inspect installed binaries at `{}`: {error}",
                bin_dir.display()
            ),
            1,
        )
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|error| {
            RunError::new(
                format!(
                    "failed to inspect installed binaries at `{}`: {error}",
                    bin_dir.display()
                ),
                1,
            )
        })?;

        let path = entry.path();
        if path.is_file() {
            entries.push(path);
        }
    }

    entries.sort();

    if entries.is_empty() {
        let package_label = package_name.unwrap_or("the selected package");
        return Err(RunError::new(
            format!("`{package_label}` did not install any executable binaries"),
            1,
        ));
    }

    if let Some(package_name) = package_name
        && let Some(entry) = entries
            .iter()
            .find(|entry| binary_target_name(entry).is_some_and(|name| name == package_name))
    {
        return Ok(entry.clone());
    }

    if entries.len() == 1 {
        return Ok(entries.remove(0));
    }

    let known = entries
        .iter()
        .map(|entry| {
            entry
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry.display().to_string())
        })
        .collect::<Vec<_>>()
        .join(", ");

    let message = if let Some(package_name) = package_name {
        format!(
            "`{package_name}` installed multiple binaries ({known}), unable to select one automatically"
        )
    } else {
        format!("installed multiple binaries ({known}), unable to select one automatically")
    };

    Err(RunError::new(message, 1))
}

#[cfg(windows)]
fn binary_target_name(path: &Path) -> Option<&str> {
    path.file_stem()?.to_str()
}

#[cfg(not(windows))]
fn binary_target_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}

fn exit_code_from_status(status: &ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
}

#[derive(Debug)]
struct TempInstallRoot {
    path: PathBuf,
}

impl TempInstallRoot {
    fn new() -> io::Result<Self> {
        let base = resolve_dlx_directories()?.temp_base_dir().to_path_buf();
        fs::create_dir_all(&base)?;

        let timestamp_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        for suffix in 0..100 {
            let candidate = base.join(format!("{timestamp_nanos}-{suffix}"));

            match fs::create_dir(&candidate) {
                Ok(()) => return Ok(Self { path: candidate }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a temporary install root",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn bin_dir(&self) -> PathBuf {
        self.path.join("bin")
    }
}

impl Drop for TempInstallRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
    };

    use clap::Parser;

    use super::super::cli::Cli;
    use super::{
        binary_target_name, package_cache_dir, resolve_clear_directories_with,
        resolve_dlx_directories_with, resolve_executable,
    };

    #[test]
    fn picks_single_binary_when_name_is_different() {
        let temp_dir = new_temp_dir("single-binary");
        let bin_dir = temp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let bin_name = if cfg!(windows) {
            "custom-runner.exe"
        } else {
            "custom-runner"
        };

        fs::write(bin_dir.join(bin_name), b"").unwrap();

        let executable = resolve_executable(&bin_dir, Some("my-crate")).unwrap();
        assert_eq!(binary_target_name(&executable), Some("custom-runner"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn picks_binary_matching_package_name() {
        let temp_dir = new_temp_dir("matching-name");
        let bin_dir = temp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let first_name = if cfg!(windows) { "alpha.exe" } else { "alpha" };
        let second_name = if cfg!(windows) { "tool.exe" } else { "tool" };

        fs::write(bin_dir.join(first_name), b"").unwrap();
        fs::write(bin_dir.join(second_name), b"").unwrap();

        let executable = resolve_executable(&bin_dir, Some("tool")).unwrap();
        assert_eq!(binary_target_name(&executable), Some("tool"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn rejects_ambiguous_binary_set() {
        let temp_dir = new_temp_dir("ambiguous");
        let bin_dir = temp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let first_name = if cfg!(windows) { "alpha.exe" } else { "alpha" };
        let second_name = if cfg!(windows) { "beta.exe" } else { "beta" };

        fs::write(bin_dir.join(first_name), b"").unwrap();
        fs::write(bin_dir.join(second_name), b"").unwrap();

        let error = resolve_executable(&bin_dir, Some("tool")).unwrap_err();
        assert!(error.to_string().contains("installed multiple binaries"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn package_cache_dir_prefers_cli_cache_dir() {
        let cli = Cli::parse_from([
            "cargo-dlx",
            "--cache-dir",
            "/tmp/cargo-dlx-package-cache",
            "ripgrep",
        ]);

        assert_eq!(
            package_cache_dir(&cli).unwrap(),
            Some(PathBuf::from("/tmp/cargo-dlx-package-cache"))
        );
    }

    #[test]
    fn package_cache_dir_is_disabled_by_flag() {
        let cli = Cli::parse_from(["cargo-dlx", "--no-package-cache", "ripgrep"]);
        assert_eq!(package_cache_dir(&cli).unwrap(), None);
    }

    #[test]
    fn resolves_dlx_directories_from_home_by_default() {
        let directories = resolve_dlx_directories_with(
            Path::new("workspace"),
            None,
            None,
            None,
            Some(PathBuf::from("home")),
        )
        .unwrap();

        assert_eq!(directories.root, PathBuf::from("home").join(".cargo-dlx"));
        assert_eq!(
            directories.temp_base,
            PathBuf::from("home").join(".cargo-dlx").join("tmp")
        );
        assert_eq!(
            directories.build_target_dir(),
            PathBuf::from("home")
                .join(".cargo-dlx")
                .join("build")
                .join("target")
        );
    }

    #[test]
    fn resolves_dlx_directories_from_root_override() {
        let directories = resolve_dlx_directories_with(
            Path::new("workspace"),
            Some(OsString::from("custom-root")),
            None,
            None,
            Some(PathBuf::from("home")),
        )
        .unwrap();

        assert_eq!(
            directories.root,
            PathBuf::from("workspace").join("custom-root")
        );
        assert_eq!(
            directories.temp_base,
            PathBuf::from("workspace").join("custom-root").join("tmp")
        );
        assert_eq!(
            directories.build_target_dir(),
            PathBuf::from("workspace")
                .join("custom-root")
                .join("build")
                .join("target")
        );
    }

    #[test]
    fn resolves_dlx_directories_with_temp_and_build_overrides() {
        let directories = resolve_dlx_directories_with(
            Path::new("workspace"),
            Some(OsString::from("custom-root")),
            Some(OsString::from("runtime-temp")),
            Some(OsString::from("build-cache")),
            Some(PathBuf::from("home")),
        )
        .unwrap();

        assert_eq!(
            directories.temp_base,
            PathBuf::from("workspace").join("runtime-temp")
        );
        assert_eq!(
            directories.build_target_dir(),
            PathBuf::from("workspace")
                .join("build-cache")
                .join("target")
        );
    }

    #[test]
    fn errors_when_no_root_or_home_is_available() {
        let error = resolve_dlx_directories_with(Path::new("workspace"), None, None, None, None)
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("could not determine cargo-dlx root directory")
        );
    }

    #[test]
    fn resolve_clear_directories_allows_temp_and_build_overrides_without_root() {
        let directories = resolve_clear_directories_with(
            Path::new("workspace"),
            None,
            None,
            Some(OsString::from("runtime-temp")),
            Some(OsString::from("build-cache")),
            None,
        )
        .unwrap();

        assert_eq!(
            directories.temp_base,
            PathBuf::from("workspace").join("runtime-temp")
        );
        assert_eq!(
            directories.build_target,
            PathBuf::from("workspace")
                .join("build-cache")
                .join("target")
        );
    }

    #[test]
    fn resolve_clear_directories_allows_explicit_cache_dir_without_root() {
        let directories = resolve_clear_directories_with(
            Path::new("workspace"),
            Some(PathBuf::from("explicit-cache")),
            None,
            Some(OsString::from("runtime-temp")),
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            directories.temp_base,
            PathBuf::from("workspace").join("runtime-temp")
        );
        assert_eq!(directories.build_target, PathBuf::from("explicit-cache"));
    }

    fn new_temp_dir(label: &str) -> PathBuf {
        let mut base = std::env::temp_dir();
        base.push(format!(
            "cargo-dlx-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));

        base
    }
}
