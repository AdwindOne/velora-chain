use clap::Parser;
use velora_consensus::poa::Poa;
use velora_consensus::Consensus;
use velora_executor::Executor;
use velora_network::Network;
use velora_rpc::{run_server, RpcContext};
use std::path::PathBuf;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use anyhow::Result;
use velora_core::genesis::Genesis;
use std::fs;
use velora_core::types::Address;
use velora_core::{Block, Transaction, BlockHeader};
use log::{info, warn, error};
use sha3::Digest;

/// Velora-chain: A modular, high-performance Rust blockchain for EVM-compatible smart contracts.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Run the Velora node
    Run(RunArgs),
    /// Initialize the Velora node
    Init(InitArgs),
}

#[derive(Parser, Debug)]
pub struct RunArgs {
    #[clap(long, value_name = "PATH", default_value = ".velora", help = "Path to the data directory")]
    pub datadir: PathBuf,
    #[clap(long, value_name = "PORT", default_value = "30303", help = "P2P listening port")]
    pub p2p_port: u16,
    #[clap(long, value_name = "ADDR", default_value = "127.0.0.1:8545", help = "RPC server address")]
    pub rpc_addr: SocketAddr,
    #[clap(long, value_name = "MS", default_value = "5000", help = "Block production interval in milliseconds")]
    pub block_time: u64,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    #[clap(long, value_name = "PATH", default_value = ".velora", help = "Path to the data directory")]
    pub datadir: PathBuf,
    #[clap(long, value_name = "PATH", help = "Path to the genesis file")]
    pub genesis: PathBuf,
}

struct App {
    executor: Arc<Executor>,
    network: Arc<Mutex<Network>>,
    consensus: Arc<dyn Consensus>,
    tx_pool: Arc<Mutex<Vec<Transaction>>>,
}

pub async fn run_node(args: RunArgs) -> Result<()> {
    let genesis_path = args.datadir.join("genesis.json");
    let db_path = args.datadir.join("db");

    if !genesis_path.exists() {
        error!("Genesis file not found at {:?}. Please run `init` first.", genesis_path);
        return Ok(());
    }

    let genesis: Genesis = serde_json::from_str(&fs::read_to_string(genesis_path)?)?;
    let validators: Vec<Address> = genesis.config.poa.as_ref().map_or_else(Vec::new, |p| p.validators.clone());

    let executor = Arc::new(Executor::new(&db_path)?);
    if executor.get_latest_block().is_err() {
        info!("Applying genesis block...");
        executor.apply_genesis(&genesis)?;
    }

    let (block_sender, mut block_receiver) = mpsc::unbounded_channel::<Block>();
    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel::<Transaction>();

    let network = Arc::new(Mutex::new(Network::new(args.p2p_port, block_sender, tx_sender.clone()).await?));

    let rpc_context = Arc::new(RpcContext {
        executor: executor.clone(),
        tx_sender: tx_sender.clone(),
    });
    let (rpc_handle, rpc_addr) = run_server(rpc_context, args.rpc_addr).await.map_err(|e| anyhow::anyhow!(e))?;

    info!("P2P listening on port: {}", args.p2p_port);
    info!("RPC server listening on: {}", rpc_addr);

    let app = Arc::new(App {
        executor: executor.clone(),
        network: network.clone(),
        consensus: Arc::new(Poa::new(validators)),
        tx_pool: Arc::new(Mutex::new(Vec::new())),
    });

    let mut block_time = interval(Duration::from_millis(args.block_time));

    let network_handle = tokio::spawn(async move {
        let binding = network.clone();
        let mut network_locked = binding.lock().await;
        network_locked.run().await;
    });

    // Main event loop
    loop {
        tokio::select! {
            _ = block_time.tick() => {
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(e) = create_and_process_block(app).await {
                        error!("Failed to create block: {}", e);
                    }
                });
            },
            Some(block) = block_receiver.recv() => {
                info!("Received new block {} from network", block.header.number);
                if let Err(e) = app.executor.apply_block(&block) {
                    warn!("Failed to apply block from network: {}", e);
                }
            },
            Some(tx) = tx_receiver.recv() => {
                info!("Received new transaction from network or RPC");
                app.tx_pool.lock().await.push(tx);
            },
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                rpc_handle.stop().unwrap();
                network_handle.abort();
                break;
            }
        }
    }

    Ok(())
}

async fn create_and_process_block(app: Arc<App>) -> Result<()> {
    let parent = app.executor.get_latest_block()?;
    let mut header = app.consensus.prepare_header(&parent.header).await?;

    let mut txs = app.tx_pool.lock().await;
    let block_txs = txs.drain(..).collect::<Vec<_>>();

    header.transactions_root = calculate_transactions_root(&block_txs);

    let mut block = Block {
        header,
        transactions: block_txs,
        ommers: vec![],
    };

    app.consensus.finalize_block(&mut block).await?;
    app.executor.apply_block(&block)?;

    info!("Produced new block {}", block.header.number);

    app.network.lock().await.broadcast_block(&block)?;

    Ok(())
}

fn calculate_transactions_root(txs: &[Transaction]) -> velora_core::H256 {
    // Placeholder. A real implementation would build a Merkle Trie.
    if txs.is_empty() {
        return velora_core::H256::zero();
    }
    let mut hasher = sha3::Keccak256::new();
    for tx in txs {
        hasher.update(tx.hash.as_bytes());
    }
    velora_core::H256::from_slice(&hasher.finalize())
}

pub fn init_node(args: InitArgs) -> Result<()> {
    fs::create_dir_all(&args.datadir)?;
    let db_path = args.datadir.join("db");
    if !db_path.exists() {
        fs::create_dir(&db_path)?;
    }
    fs::copy(args.genesis, args.datadir.join("genesis.json"))?;
    info!("Node initialized at: {:?}", args.datadir);
    Ok(())
}
