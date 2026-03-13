use clap::Parser;

use cargo_dlx::ops::cargo_dlx;

cargo_subcommand_metadata::description!("Download, compile, and execute a Rust binary in one shot");

fn main() {
    let mut argv = std::env::args_os();
    let program_name = argv
        .next()
        .unwrap_or_else(|| std::ffi::OsString::from("cargo-dlx"));
    let raw_args = cargo_dlx::Cli::normalize_raw_args(argv);

    let cmd =
        cargo_dlx::Cli::parse_from(std::iter::once(program_name).chain(raw_args.iter().cloned()));
    if let Err(error) = cmd.validate() {
        error.exit();
    }

    match cargo_dlx::execute(&cmd) {
        Ok(cargo_dlx::Execution::Completed) => {}
        Ok(cargo_dlx::Execution::ChildExited(code)) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
