#[derive(Debug, Clone, clap::Parser)]
#[command(name = "cargo dlx")]
#[command(bin_name = "cargo dlx")]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
pub struct Cli {
    #[command(flatten)]
    manifest: clap_cargo::Manifest,
    #[command(flatten)]
    workspace: clap_cargo::Workspace,
    #[command(flatten)]
    features: clap_cargo::Features,
}
