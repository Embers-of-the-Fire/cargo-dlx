use clap::Parser;

mod cli;

fn main() {
    let cmd = cli::Cli::parse();
    println!("Hello, world!");
    println!("{:?}", cmd);
}
