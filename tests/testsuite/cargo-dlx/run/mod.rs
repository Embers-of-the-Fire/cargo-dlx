use cargo_test_macro::cargo_test;
use cargo_test_support::{project, registry::Package};

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
