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
fn runs_relative_file_url_binary_from_reference() {
    let p = project()
        .file(
            "src/main.rs",
            r#"
fn main() {}
"#,
        )
        .file(
            "dlx-relative-file-source/Cargo.toml",
            r#"
[package]
name = "dlx-relative-file-source"
version = "0.1.0"
edition = "2021"
"#,
        )
        .file(
            "dlx-relative-file-source/src/main.rs",
            r#"
fn main() {
    println!("hello from relative file ref");
}
"#,
        )
        .build();

    p.cargo_dlx("file://dlx-relative-file-source")
        .with_stdout_contains("hello from relative file ref")
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
