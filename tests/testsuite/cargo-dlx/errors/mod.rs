use cargo_test_macro::cargo_test;
use cargo_test_support::{project, str};

use crate::ProjectExt;

#[cargo_test]
fn rejects_prefixed_semver_version() {
    let p = project().build();

    p.cargo_dlx("ripgrep@v14.1.1")
        .with_status(1)
        .with_stdout_data(str![""])
        .with_stderr_contains(
            "[ERROR] the version provided, `v14.1.1` is not a valid SemVer requirement",
        )
        .with_stderr_contains("[HELP] try changing the version to `14.1.1`")
        .run();
}

#[cargo_test]
fn strips_cargo_subcommand_prefix() {
    let p = project().build();

    p.cargo_dlx("dlx ripgrep@v14.1.1")
        .env("CARGO", "cargo")
        .with_status(1)
        .with_stdout_data(str![""])
        .with_stderr_contains(
            "[ERROR] the version provided, `v14.1.1` is not a valid SemVer requirement",
        )
        .with_stderr_contains("[HELP] try changing the version to `14.1.1`")
        .run();
}

#[cargo_test]
fn rejects_bin_without_name() {
    let p = project().build();

    p.cargo_dlx("--bin -- ripgrep")
        .with_status(2)
        .with_stdout_data(str![""])
        .with_stderr_contains("[..]a value is required[..]")
        .with_stderr_contains("[..]--bin <NAME>[..]")
        .run();
}

#[cargo_test]
fn rejects_example_without_name() {
    let p = project().build();

    p.cargo_dlx("--example -- ripgrep")
        .with_status(2)
        .with_stdout_data(str![""])
        .with_stderr_contains("[..]a value is required[..]")
        .with_stderr_contains("[..]--example <NAME>[..]")
        .run();
}
