use clap::{CommandFactory, Parser};

mod cli;
mod run;

cargo_subcommand_metadata::description!("Download, compile, and execute a Rust binary in one shot");

fn main() {
    let raw_args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if cli::Cli::wants_help(raw_args.iter().cloned()) {
        let mut command = cli::Cli::command();
        command
            .print_help()
            .expect("failed to write `cargo dlx` help output");
        println!();
        return;
    }

    let cmd = cli::Cli::parse();
    if let Err(error) = cmd.validate() {
        error.exit();
    }

    match run::execute(&cmd) {
        Ok(run::Execution::Completed) => {}
        Ok(run::Execution::ChildExited(code)) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
