use cargo_test_macro::cargo_test;
use cargo_test_support::{file, str};

use crate::CargoCommandExt;

#[cargo_test]
fn case() {
    snapbox::cmd::Command::cargo_ui()
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.term.svg"])
        .stderr_eq(str![""]);
}
