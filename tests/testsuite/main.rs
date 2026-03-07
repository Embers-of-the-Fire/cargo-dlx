mod cargo_dlx;

use cargo_test_support::{execs, process, ArgLineCommandExt, Execs, Project, TestEnvCommandExt};
use cargo_util::ProcessBuilder;

pub fn cargo_dlx_exe() -> std::path::PathBuf {
    snapbox::cmd::cargo_bin!("cargo-dlx").to_path_buf()
}

pub trait ProjectExt {
    /// Creates an `Execs` instance to run the `cargo-dlx` binary
    fn cargo_dlx(&self, cmd: &str) -> Execs;
    /// Creates an `Execs` instance to run the globally installed `cargo` command
    fn cargo_global(&self, cmd: &str) -> Execs;
}

impl ProjectExt for Project {
    fn cargo_dlx(&self, cmd: &str) -> Execs {
        let cargo_dlx = cargo_dlx_exe();

        let mut p = process(&cargo_dlx);
        p.cwd(self.root()).arg_line(cmd);

        execs().with_process_builder(p)
    }

    fn cargo_global(&self, cmd: &str) -> Execs {
        let cargo = std::env::var_os("CARGO").unwrap_or("cargo".into());

        let mut p = ProcessBuilder::new(cargo);
        p.test_env().cwd(self.root()).arg_line(cmd);

        execs().with_process_builder(p)
    }
}

pub trait CargoCommandExt {
    fn cargo_ui() -> Self;
}

impl CargoCommandExt for snapbox::cmd::Command {
    fn cargo_ui() -> Self {
        Self::new(cargo_dlx_exe())
            .with_assert(cargo_test_support::compare::assert_ui())
            .env("CARGO_TERM_COLOR", "always")
            .env("CARGO_TERM_HYPERLINKS", "true")
            .test_env()
    }
}
