use std::fs;

use cargo_test_macro::cargo_test;
use cargo_test_support::{git, paths::CargoPathExt, project, registry::Package};

use crate::ProjectExt;

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
        .with_stdout_contains("hello from cargo-dlx")
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
        .with_stdout_contains("--help|--color|always")
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
        .with_stderr_contains("dlx-exit failed intentionally")
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
        .with_stdout_contains("hello from git ref")
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
        .with_stdout_contains("hello from file ref")
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
        .with_stdout_contains("hello from path+file ref")
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
        .with_stdout_contains("hello from registry ref")
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
        .with_stdout_contains("hello from root defaults")
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
        .with_stdout_contains("hello from root overrides")
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

fn temp_dir_is_empty(path: &std::path::Path) -> bool {
    fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}
