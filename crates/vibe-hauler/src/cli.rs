use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "vhaul", version, about = "Launch the VibeHauler TUI.")]
pub struct Cli {
    #[arg(long)]
    pub config: Option<String>,

    #[arg(long)]
    pub no_color: bool,

    #[arg(long)]
    pub portable_root: Option<String>,

    #[arg(long)]
    pub roots: Option<String>,
}

#[allow(clippy::unnecessary_wraps)]
pub fn run() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("VibeHauler TUI scaffold: interactive cleanup is pending.");
    Ok(())
}
