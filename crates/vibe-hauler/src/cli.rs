use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "vhaul", version, about = "Haul away your agent clutter.")]
pub struct Cli {
    #[arg(long, global = true)]
    pub config: Option<String>,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub no_color: bool,

    #[arg(long, global = true)]
    pub portable_root: Option<String>,

    #[arg(long, global = true)]
    pub apps: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Scan(ScanArgs),
    Sessions(SessionsArgs),
    Plan(PlanArgs),
    Clean(CleanArgs),
    Restore(RestoreArgs),
    Doctor,
    Completion(CompletionArgs),
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    #[arg(long)]
    pub roots: Option<String>,
}

#[derive(Debug, Args)]
pub struct SessionsArgs {
    #[command(subcommand)]
    pub command: SessionsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionsCommand {
    List(SessionListArgs),
    Show(SessionShowArgs),
    Export(SessionExportArgs),
}

#[derive(Debug, Args)]
pub struct SessionListArgs {
    #[arg(long)]
    pub app: Option<String>,

    #[arg(long)]
    pub since: Option<String>,

    #[arg(long)]
    pub cwd: Option<String>,

    #[arg(long)]
    pub sort: Option<String>,
}

#[derive(Debug, Args)]
pub struct SessionShowArgs {
    pub session_id: String,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Args)]
pub struct SessionExportArgs {
    pub session_id: String,

    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,

    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    #[arg(long)]
    pub safe: bool,

    #[arg(long)]
    pub include_yellow: bool,

    #[arg(long)]
    pub older_than: Option<String>,
}

#[derive(Debug, Args)]
pub struct CleanArgs {
    #[arg(long)]
    pub plan: Option<String>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub execute: bool,

    #[arg(long)]
    pub safe_only: bool,
}

#[derive(Debug, Args)]
pub struct RestoreArgs {
    pub manifest: String,
}

#[derive(Debug, Args)]
pub struct CompletionArgs {
    pub shell: String,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Markdown,
    Json,
    Jsonl,
}

#[allow(clippy::unnecessary_wraps)]
pub fn run() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("VibeHauler scaffold: command wiring is ready; implementation is pending.");
    Ok(())
}
