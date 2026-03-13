use std::fs;

use cargo_test_macro::cargo_test;
use cargo_test_support::{git, paths::CargoPathExt, project, registry::Package, str};

use crate::ProjectExt;

#[cargo_test]
fn rejects_prefixed_semver_version() {
    let p = project().build();

    p.cargo_dlx("ripgrep@v14.1.1")
        .with_status(1)
        .with_stdout_data("")
        .with_stderr_data(str![[r#"
[ERROR] the version provided, `v14.1.1` is not a valid SemVer requirement

[HELP] try changing the version to `14.1.1`

"#]])
        .run();
}

#[cargo_test]
fn strips_cargo_subcommand_prefix() {
    let p = project().build();

    p.cargo_dlx("dlx ripgrep@v14.1.1")
        .env("CARGO", "cargo")
        .with_status(1)
        .with_stdout_data("")
        .with_stderr_data(str![[r#"
[ERROR] the version provided, `v14.1.1` is not a valid SemVer requirement

[HELP] try changing the version to `14.1.1`

"#]])
        .run();
}

#[cargo_test]
fn runs_registry_binary() {
    Package::new("dlx-hello", "0.1.0")
        .file(
            "src/main.rs",
            r#"
fn main() {
    println!("hello from cargo-dlx");
}
"#,
        )
        .publish();

    let p = project().build();

    p.cargo_dlx("dlx-hello")
        .with_stdout_data(str![[r#"
hello from cargo-dlx

"#]])
        .run();
}

#[cargo_test]
fn forwards_args_to_installed_binary() {
    Package::new("dlx-args", "0.1.0")
        .file(
            "src/main.rs",
            r#"
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    println!("{}", args.join("|"));
}
"#,
        )
        .publish();

    let p = project().build();

    p.cargo_dlx("dlx-args --help --color always")
        .with_stdout_data(str![[r#"
--help|--color|always

"#]])
        .run();
}

#[cargo_test]
fn returns_binary_exit_code() {
    Package::new("dlx-exit", "0.1.0")
        .file(
            "src/main.rs",
            r#"
fn main() {
    eprintln!("dlx-exit failed intentionally");
    std::process::exit(42);
}
"#,
        )
        .publish();

    let p = project().build();

    p.cargo_dlx("dlx-exit")
        .with_status(42)
        .with_stdout_data("")
        .with_stderr_data(str![[r#"
...
dlx-exit failed intentionally

"#]])
        .run();
}

#[cargo_test]
fn runs_git_binary_from_reference() {
    let (git_project, git_repo) = git::new_repo("dlx-git-ref", |project| {
        project
            .file(
                "Cargo.toml",
                r#"
[package]
name = "dlx-git-ref"
version = "0.1.0"
edition = "2021"
"#,
            )
            .file(
                "src/main.rs",
                r#"
fn main() {
    println!("hello from git ref");
}
"#,
            )
    });
    let rev = git_repo.revparse_single("HEAD").unwrap().id();

    let p = project().build();

    p.cargo_dlx(&format!("git+{}?rev={rev}#dlx-git-ref", git_project.url()))
        .with_stdout_data(str![[r#"
hello from git ref

"#]])
        .run();
}

#[cargo_test]
fn runs_file_binary_from_reference() {
    let source = project()
        .at("dlx-file-source")
        .file(
            "Cargo.toml",
            r#"
[package]
name = "dlx-file-source"
version = "0.1.0"
edition = "2021"
"#,
        )
        .file(
            "src/main.rs",
            r#"
fn main() {
    println!("hello from file ref");
}
"#,
        )
        .build();

    let p = project().build();

    p.cargo_dlx(&format!("{}#dlx-file-source", source.root().to_url()))
        .with_stdout_data(str![[r#"
hello from file ref

"#]])
        .run();
}

#[cargo_test]
fn runs_path_file_binary_from_reference() {
    let source = project()
        .at("dlx-path-source")
        .file(
            "Cargo.toml",
            r#"
[package]
name = "dlx-path-source"
version = "0.1.0"
edition = "2021"
"#,
        )
        .file(
            "src/main.rs",
            r#"
fn main() {
    println!("hello from path+file ref");
}
"#,
        )
        .build();

    let p = project().build();

    p.cargo_dlx(&format!("path+{}#dlx-path-source", source.root().to_url()))
        .with_stdout_data(str![[r#"
hello from path+file ref

"#]])
        .run();
}

#[cargo_test]
fn runs_registry_index_binary_from_reference() {
    Package::new("dlx-registry-ref", "0.1.0")
        .file(
            "src/main.rs",
            r#"
fn main() {
    println!("hello from registry ref");
}
"#,
        )
        .publish();

    let p = project().build();

    p.cargo_dlx("registry+https://github.com/rust-lang/crates.io-index#dlx-registry-ref@0.1.0")
        .with_stdout_data(str![[r#"
hello from registry ref

"#]])
        .run();
}

#[cargo_test]
fn uses_root_for_temp_and_build_by_default() {
    Package::new("dlx-root-defaults", "0.1.0")
        .file(
            "src/main.rs",
            r#"
fn main() {
    println!("hello from root defaults");
}
"#,
        )
        .publish();

    let p = project().build();
    let dlx_root = p.root().join(".test-cargo-dlx-root");

    p.cargo_dlx("dlx-root-defaults")
        .env("CARGO_DLX_ROOT", &dlx_root)
        .with_stdout_data(str![[r#"
hello from root defaults

"#]])
        .run();

    let temp_base = dlx_root.join("tmp");
    let build_target = dlx_root.join("build").join("target");

    assert!(
        temp_base.exists(),
        "expected temp base at {}",
        temp_base.display()
    );
    assert!(
        temp_dir_is_empty(&temp_base),
        "expected temp install roots to be cleaned up under {}",
        temp_base.display()
    );
    assert!(
        build_target.exists(),
        "expected build target cache at {}",
        build_target.display()
    );
}

#[cargo_test]
fn uses_temp_and_build_overrides_instead_of_root_derived_paths() {
    Package::new("dlx-root-overrides", "0.1.0")
        .file(
            "src/main.rs",
            r#"
fn main() {
    println!("hello from root overrides");
}
"#,
        )
        .publish();

    let p = project().build();
    let dlx_root = p.root().join(".test-cargo-dlx-root-unused");
    let dlx_temp = p.root().join(".test-cargo-dlx-temp");
    let dlx_build = p.root().join(".test-cargo-dlx-build");

    p.cargo_dlx("dlx-root-overrides")
        .env("CARGO_DLX_ROOT", &dlx_root)
        .env("CARGO_DLX_TEMP", &dlx_temp)
        .env("CARGO_DLX_BUILD", &dlx_build)
        .with_stdout_data(str![[r#"
hello from root overrides

"#]])
        .run();

    let root_temp = dlx_root.join("tmp");
    let root_build = dlx_root.join("build").join("target");
    let override_build = dlx_build.join("target");

    assert!(
        dlx_temp.exists(),
        "expected temp base at {}",
        dlx_temp.display()
    );
    assert!(
        temp_dir_is_empty(&dlx_temp),
        "expected temp install roots to be cleaned up under {}",
        dlx_temp.display()
    );
    assert!(
        override_build.exists(),
        "expected build target cache at {}",
        override_build.display()
    );

    assert!(
        !root_temp.exists(),
        "did not expect root-derived temp path at {}",
        root_temp.display()
    );
    assert!(
        !root_build.exists(),
        "did not expect root-derived build path at {}",
        root_build.display()
    );
}

#[cargo_test]
fn clear_removes_root_temp_and_build_cache() {
    let p = project().build();
    let dlx_root = p.root().join(".test-cargo-dlx-clear-root");
    let temp_base = dlx_root.join("tmp");
    let build_target = dlx_root.join("build").join("target");

    fs::create_dir_all(temp_base.join("stale")).unwrap();
    fs::create_dir_all(&build_target).unwrap();
    fs::write(temp_base.join("stale").join("artifact"), b"x").unwrap();
    fs::write(build_target.join("artifact"), b"x").unwrap();

    p.cargo_dlx("--clear")
        .env("CARGO_DLX_ROOT", &dlx_root)
        .with_stdout_data("")
        .with_stderr_data(str![])
        .run();

    assert!(!temp_base.exists());
    assert!(!build_target.exists());
}

#[cargo_test]
fn clear_respects_temp_and_build_overrides() {
    let p = project().build();
    let dlx_root = p.root().join(".test-cargo-dlx-clear-root-unused");
    let dlx_temp = p.root().join(".test-cargo-dlx-clear-temp");
    let dlx_build = p.root().join(".test-cargo-dlx-clear-build");

    let root_temp = dlx_root.join("tmp");
    let root_build_target = dlx_root.join("build").join("target");
    let override_build_target = dlx_build.join("target");

    fs::create_dir_all(root_temp.join("keep")).unwrap();
    fs::create_dir_all(&root_build_target).unwrap();
    fs::create_dir_all(dlx_temp.join("stale")).unwrap();
    fs::create_dir_all(&override_build_target).unwrap();

    fs::write(root_temp.join("keep").join("artifact"), b"x").unwrap();
    fs::write(root_build_target.join("artifact"), b"x").unwrap();
    fs::write(dlx_temp.join("stale").join("artifact"), b"x").unwrap();
    fs::write(override_build_target.join("artifact"), b"x").unwrap();

    p.cargo_dlx("--clear")
        .env("CARGO_DLX_ROOT", &dlx_root)
        .env("CARGO_DLX_TEMP", &dlx_temp)
        .env("CARGO_DLX_BUILD", &dlx_build)
        .with_stdout_data("")
        .with_stderr_data(str![])
        .run();

    assert!(!dlx_temp.exists());
    assert!(!override_build_target.exists());
    assert!(root_temp.exists());
    assert!(root_build_target.exists());
}

#[cargo_test]
fn clear_works_with_temp_and_build_overrides_without_root_or_home() {
    let p = project().build();
    let dlx_temp = p.root().join(".test-cargo-dlx-clear-no-home-temp");
    let dlx_build = p.root().join(".test-cargo-dlx-clear-no-home-build");
    let build_target = dlx_build.join("target");

    fs::create_dir_all(dlx_temp.join("stale")).unwrap();
    fs::create_dir_all(&build_target).unwrap();
    fs::write(dlx_temp.join("stale").join("artifact"), b"x").unwrap();
    fs::write(build_target.join("artifact"), b"x").unwrap();

    p.cargo_dlx("--clear")
        .env("CARGO_DLX_TEMP", &dlx_temp)
        .env("CARGO_DLX_BUILD", &dlx_build)
        .env("HOME", "")
        .env("USERPROFILE", "")
        .env("HOMEDRIVE", "")
        .env("HOMEPATH", "")
        .with_stdout_data("")
        .with_stderr_data(str![])
        .run();

    assert!(!dlx_temp.exists());
    assert!(!build_target.exists());
}

#[cargo_test]
fn clear_works_with_temp_and_explicit_cache_without_root_or_home() {
    let p = project().build();
    let dlx_temp = p.root().join(".test-cargo-dlx-clear-no-home-temp");
    let explicit_cache_dir = p.root().join("explicit-cache");

    fs::create_dir_all(dlx_temp.join("stale")).unwrap();
    fs::create_dir_all(&explicit_cache_dir).unwrap();
    fs::write(dlx_temp.join("stale").join("artifact"), b"x").unwrap();
    fs::write(explicit_cache_dir.join("artifact"), b"x").unwrap();

    p.cargo_dlx("--clear --cache-dir explicit-cache")
        .env("CARGO_DLX_TEMP", &dlx_temp)
        .env("HOME", "")
        .env("USERPROFILE", "")
        .env("HOMEDRIVE", "")
        .env("HOMEPATH", "")
        .with_stdout_data("")
        .with_stderr_data(str![])
        .run();

    assert!(!dlx_temp.exists());
    assert!(!explicit_cache_dir.exists());
}

#[cargo_test]
fn clear_rejects_package_arguments() {
    let p = project().build();

    p.cargo_dlx("--clear ripgrep")
        .with_status(2)
        .with_stdout_data("")
        .with_stderr_data(str![[r#"
[ERROR] the argument '--clear' cannot be used with '[CRATE[@<VER>]] [ARG]...'

Usage: cargo dlx --clear [CRATE[@<VER>]] [ARG]...

For more information, try '--help'.

"#]])
        .run();
}

#[cargo_test]
fn clear_uses_explicit_cache_dir_instead_of_root_build_cache() {
    let p = project().build();
    let dlx_root = p.root().join(".test-cargo-dlx-clear-explicit-root");
    let root_build_target = dlx_root.join("build").join("target");
    let explicit_cache_dir = p.root().join("explicit-cache");

    fs::create_dir_all(&root_build_target).unwrap();
    fs::create_dir_all(&explicit_cache_dir).unwrap();
    fs::write(root_build_target.join("artifact"), b"x").unwrap();
    fs::write(explicit_cache_dir.join("artifact"), b"x").unwrap();

    p.cargo_dlx("--clear --cache-dir explicit-cache")
        .env("CARGO_DLX_ROOT", &dlx_root)
        .with_stdout_data("")
        .with_stderr_data(str![])
        .run();

    assert!(!explicit_cache_dir.exists());
    assert!(root_build_target.exists());
}

fn temp_dir_is_empty(path: &std::path::Path) -> bool {
    fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}
