use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    time::{SystemTime, UNIX_EPOCH},
};

use super::cli::{Cli, CrateSpec, GitReference, PackageSource};

const CARGO_DLX_CACHE_DIR_ENV: &str = "CARGO_DLX_CACHE_DIR";

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
    let Some(cache_dir) = package_cache_dir(cli) else {
        return Ok(());
    };

    fs::create_dir_all(&cache_dir)?;
    command.env("CARGO_TARGET_DIR", cache_dir);

    Ok(())
}

fn package_cache_dir(cli: &Cli) -> Option<PathBuf> {
    if cli.no_package_cache {
        return None;
    }

    if let Some(path) = &cli.cache_dir {
        return Some(path.clone());
    }

    package_cache_dir_from_env().or_else(default_package_cache_dir)
}

fn package_cache_dir_from_env() -> Option<PathBuf> {
    non_empty_env_os(CARGO_DLX_CACHE_DIR_ENV).map(PathBuf::from)
}

#[cfg(windows)]
fn default_package_cache_dir() -> Option<PathBuf> {
    if let Some(local_app_data) = non_empty_env_os("LOCALAPPDATA") {
        return Some(
            PathBuf::from(local_app_data)
                .join("cargo-dlx")
                .join("target"),
        );
    }

    non_empty_env_os("USERPROFILE").map(|user_profile| {
        PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("cargo-dlx")
            .join("target")
    })
}

#[cfg(not(windows))]
fn default_package_cache_dir() -> Option<PathBuf> {
    if let Some(cache_home) = non_empty_env_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(cache_home).join("cargo-dlx").join("target"));
    }

    non_empty_env_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".cache")
            .join("cargo-dlx")
            .join("target")
    })
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
        let mut base = std::env::temp_dir();
        base.push("cargo-dlx");
        fs::create_dir_all(&base)?;

        let timestamp_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        for suffix in 0..100 {
            let candidate = base.join(format!("{}-{timestamp_nanos}-{suffix}", std::process::id()));

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
    use std::{fs, path::PathBuf};

    use clap::Parser;

    use super::super::cli::Cli;
    use super::{binary_target_name, package_cache_dir, resolve_executable};

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
            package_cache_dir(&cli),
            Some(PathBuf::from("/tmp/cargo-dlx-package-cache"))
        );
    }

    #[test]
    fn package_cache_dir_is_disabled_by_flag() {
        let cli = Cli::parse_from(["cargo-dlx", "--no-package-cache", "ripgrep"]);
        assert_eq!(package_cache_dir(&cli), None);
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
