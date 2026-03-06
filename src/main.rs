use clap::Parser;

mod cli;

cargo_subcommand_metadata::description!("Run a Rust binary without installing it globally");

fn main() {
    let cmd = cli::Cli::parse();
    if let Err(error) = cmd.validate() {
        error.exit();
    }
}
