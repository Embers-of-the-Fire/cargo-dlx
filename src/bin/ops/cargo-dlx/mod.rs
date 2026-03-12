use clap::Parser;

mod cli;
mod run;

pub(crate) fn run() {
    let mut argv = std::env::args_os();
    let program_name = argv
        .next()
        .unwrap_or_else(|| std::ffi::OsString::from("cargo-dlx"));
    let raw_args = cli::Cli::normalize_raw_args(argv);

    let cmd = cli::Cli::parse_from(std::iter::once(program_name).chain(raw_args.iter().cloned()));

    match run::execute(&cmd) {
        Ok(run::Execution::Completed) => {}
        Ok(run::Execution::ChildExited(code)) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
