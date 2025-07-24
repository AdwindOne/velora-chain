use anyhow::Result;
use clap::Parser;
use velora_cli::{init_node, run_node, Args, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let args = Args::parse();

    match args.command {
        Commands::Run(run_args) => run_node(run_args).await?,
        Commands::Init(init_args) => init_node(init_args)?,
    }

    Ok(())
}
