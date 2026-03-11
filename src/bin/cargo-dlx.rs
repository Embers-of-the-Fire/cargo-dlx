use cargo_dlx::ops::cargo_dlx;

cargo_subcommand_metadata::description!("Download, compile, and execute a Rust binary in one shot");

fn main() {
    cargo_dlx::run();
}
