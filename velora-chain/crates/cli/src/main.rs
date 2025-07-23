use clap::Parser;
use velora_consensus::poa::Poa;
use velora_consensus::Consensus;
use velora_executor::Executor;
use velora_network::Network;
use velora_rpc::run_server;
use std::path::PathBuf;
use std::net::SocketAddr;
use anyhow::Result;
use velora_core::genesis::Genesis;
use std::fs;
use velora_core::types::Address;

/// Velora-chain: A modular, high-performance Rust blockchain for EVM-compatible smart contracts.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Run the Velora node
    Run(RunArgs),
    /// Initialize the Velora node
    Init(InitArgs),
}

#[derive(Parser, Debug)]
struct RunArgs {
    #[clap(long, value_name = "PATH", default_value = ".velora")]
    datadir: PathBuf,

    #[clap(long, value_name = "PORT", default_value = "30303")]
    p2p_port: u16,

    #[clap(long, value_name = "ADDR", default_value = "127.0.0.1:8545")]
    rpc_addr: SocketAddr,
}

#[derive(Parser, Debug)]
struct InitArgs {
    #[clap(long, value_name = "PATH", default_value = ".velora")]
    datadir: PathBuf,

    #[clap(long, value_name = "PATH")]
    genesis: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Run(run_args) => run_node(run_args).await?,
        Commands::Init(init_args) => init_node(init_args)?,
    }

    Ok(())
}

async fn run_node(args: RunArgs) -> Result<()> {
    let genesis_path = args.datadir.join("genesis.json");
    let db_path = args.datadir.join("db");

    let genesis: Genesis = serde_json::from_str(&fs::read_to_string(genesis_path)?)?;
    let validators: Vec<Address> = genesis.poa.map_or_else(Vec::new, |p| p.validators);

    let consensus = Poa::new(validators);
    let executor = Executor::new(&db_path)?;
    let mut network = Network::new(args.p2p_port).await?;
    let (rpc_handle, rpc_addr) = run_server(args.rpc_addr).await?;

    println!("P2P listening on port: {}", args.p2p_port);
    println!("RPC server listening on: {}", rpc_addr);

    tokio::spawn(async move {
        network.run().await;
    });

    rpc_handle.stopped().await;

    Ok(())
}

fn init_node(args: InitArgs) -> Result<()> {
    fs::create_dir_all(&args.datadir)?;
    fs::copy(args.genesis, args.datadir.join("genesis.json"))?;
    println!("Node initialized at: {:?}", args.datadir);
    Ok(())
}
