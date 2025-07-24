use anyhow::Result;
use clap::Parser;
#[allow(unused_imports)]
use velora_cli::{init_node, run_node, Args, Commands, main_entry};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let args = Args::parse();
    main_entry(args).await
}
