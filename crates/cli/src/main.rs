use clap::Parser;

/// A fast linting tool for AI agents that measures quality signals
#[derive(Parser)]
#[command(name = "quench")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    Ok(())
}
