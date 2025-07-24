use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use sha3::Digest;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use velora_consensus::poa::Poa;
use velora_consensus::Consensus;
use velora_core::genesis::Genesis;
use velora_core::types::Address;
use velora_core::{Block, Transaction};
use velora_executor::Executor;
use velora_network::Network;
use velora_rpc::{run_server, RpcContext};
use std::collections::HashMap;
use revm::db::DatabaseRef;
use revm::primitives::alloy_primitives::Address as RevmAddress;
use std::time::{SystemTime, UNIX_EPOCH};
use velora_core::H256;

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
    /// Print chain id from genesis config
    ChainId,
}

#[derive(Parser, Debug)]
pub struct RunArgs {
    #[clap(
        long,
        value_name = "PATH",
        default_value = ".velora",
        help = "Path to the data directory"
    )]
    pub datadir: PathBuf,
    #[clap(
        long,
        value_name = "PORT",
        default_value = "30303",
        help = "P2P listening port"
    )]
    pub p2p_port: u16,
    #[clap(
        long,
        value_name = "ADDR",
        default_value = "127.0.0.1:6545",
        help = "RPC server address"
    )]
    pub rpc_addr: SocketAddr,
    #[clap(
        long,
        value_name = "MS",
        default_value = "5000",
        help = "Block production interval in milliseconds"
    )]
    pub block_time: u64,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    #[clap(
        long,
        value_name = "PATH",
        default_value = ".velora",
        help = "Path to the data directory"
    )]
    pub datadir: PathBuf,
    #[clap(long, value_name = "PATH", help = "Path to the genesis file")]
    pub genesis: PathBuf,
}

struct App {
    executor: Arc<Executor>,
    network: Arc<Mutex<Network>>,
    consensus: Arc<dyn Consensus>,
    tx_pool: Arc<Mutex<HashMap<Address, Vec<Transaction>>>>,
}

//noinspection ALL
pub async fn run_node(args: RunArgs) -> Result<()> {
    let genesis_path = args.datadir.join("genesis.json");
    let db_path = args.datadir.join("db");

    if !genesis_path.exists() {
        error!(
            "Genesis file not found at {genesis_path:?}. Please run `init` first."
        );
        return Ok(());
    }

    let genesis: Genesis = serde_json::from_str(&fs::read_to_string(genesis_path)?)?;
    let validators: Vec<Address> = genesis
        .config
        .poa
        .as_ref()
        .map_or_else(Vec::new, |p| p.validators.clone());

    let executor = Arc::new(Executor::new(&db_path)?);
    if executor.get_latest_block().is_err() {
        info!("Applying genesis block...");
        executor.apply_genesis(&genesis)?;
    }

    let (block_sender, mut block_receiver) = mpsc::unbounded_channel::<Block>();
    let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel::<Transaction>();

    let network = Arc::new(Mutex::new(
        Network::new(args.p2p_port, block_sender, tx_sender.clone()).await?,
    ));

    let rpc_context = Arc::new(RpcContext {
        executor: executor.clone(),
        tx_sender: tx_sender.clone(),
    });
    let (rpc_handle, rpc_addr) = run_server(rpc_context, args.rpc_addr)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    info!("P2P listening on port: {}", args.p2p_port);
    info!("RPC server listening on: {rpc_addr}");

    let app = Arc::new(App {
        executor: executor.clone(),
        network: network.clone(),
        consensus: Arc::new(Poa::new(validators)),
        tx_pool: Arc::new(Mutex::new(HashMap::new())),
    });

    let network_handle = tokio::spawn(async move {
        let binding = network.clone();
        let mut network_locked = binding.lock().await;
        network_locked.run().await;
    });
    // block_interval_secs 直接用 args.block_time（单位秒）
    let block_interval_secs = args.block_time;
    let mut block_time = interval(Duration::from_secs(block_interval_secs));
    let  _block_handle: Option<tokio::task::JoinHandle<()>> = None;
    // Main event loop
    loop {
        tokio::select! {
            _ = block_time.tick() => {
                info!("Tick: producing block");
                info!("create_and_process_block start");
                tokio::select! {
                    res = create_and_process_block(app.clone(), block_interval_secs) => {
                        if let Err(e) = res {
                            error!("Failed to create block: {e}");
                        }
                        info!("create_and_process_block end");
                    }
                    _ = tokio::signal::ctrl_c() => {
                        info!("Shutting down...");
                        rpc_handle.stop().unwrap();
                        network_handle.abort();
                        break;
                    }
                }
            },
            Some(block) = block_receiver.recv() => {
                info!("Received new block {} from network", block.header.number);
                if let Err(e) = app.executor.apply_block(&block) {
                    warn!("Failed to apply block from network: {e}");
                }
            },
            Some(tx) = tx_receiver.recv() => {
                info!("Received new transaction from network or RPC");
                // Nonce校验和分组入池
                let mut pool = app.tx_pool.lock().await;
                let entry = pool.entry(tx.from.unwrap_or_default()).or_insert_with(Vec::new);
                // 获取链上nonce
                let chain_nonce = app.executor.basic_ref(RevmAddress::from_slice(tx.from.unwrap_or_default().as_bytes()))
                    .ok().flatten().map(|acc| acc.nonce).unwrap_or(0);
                let expected_nonce = chain_nonce + entry.len() as u64;
                if tx.nonce == expected_nonce {
                    entry.push(tx);
                } else {
                    warn!("Rejected tx: invalid nonce (got {}, expected {})", tx.nonce, expected_nonce);
                }
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

#[allow(unused_variables)]
async fn create_and_process_block(app: Arc<App>, block_interval_secs: u64) -> Result<()> {
    let parent = app.executor.get_latest_block()?;
    info!("Tick: producing block, parent number: {}, parent hash: {:?}", parent.header.number, parent.hash());
    let mut header = app.consensus.prepare_header(&parent.header).await?;
    // 强制修正关键字段，保证链持续递增
    header.parent_hash = parent.hash();
    header.number = parent.header.number + 1;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    header.timestamp = now;
    info!("create_and_process_block: locking tx_pool");
    let mut pool = app.tx_pool.lock().await;
    info!("create_and_process_block: locked tx_pool");
    let mut block_txs = Vec::new();
    // 只打包 nonce 连续的交易
    for (from, txs) in pool.iter_mut() {
        // 获取链上nonce
        let chain_nonce = app.executor.basic_ref(RevmAddress::from_slice((*from).as_bytes()))
            .ok().flatten().map(|acc| acc.nonce).unwrap_or(0);
        let mut expected_nonce = chain_nonce;
        txs.sort_by_key(|tx| tx.nonce);
        let mut i = 0;
        while i < txs.len() {
            if txs[i].nonce == expected_nonce {
                block_txs.push(txs[i].clone());
                expected_nonce += 1;
                i += 1;
            } else {
                break;
            }
        }
        // 移除已打包的交易
        txs.drain(0..i);
    }
    header.transactions_root = calculate_transactions_root(&block_txs);
    header.state_root = H256::zero();
    header.receipts_root = H256::zero();
    header.logs_bloom = velora_core::Bloom::default();
    header.difficulty = velora_core::U256::one();
    header.gas_limit = 30_000_000;
    header.gas_used = 0;
    header.mix_hash = H256::from_low_u64_be(header.timestamp);
    header.nonce = H256::from_low_u64_be(header.number);
    header.extra_data = header.timestamp.to_be_bytes().to_vec();
    header.ommers_hash = H256::zero();
    let mut block = Block {
        header,
        transactions: block_txs,
        ommers: vec![],
    };
    info!("create_and_process_block: before finalize_block");
    app.consensus.finalize_block(&mut block).await?;
    info!("create_and_process_block: after finalize_block, before apply_block");
    info!("Applying block number: {}", block.header.number);
    let res = app.executor.apply_block(&block);
    match res {
        Ok(_) => info!("Produced new block {}", block.header.number),
        Err(e) => error!("Failed to apply block {}: {}", block.header.number, e),
    }
    info!("create_and_process_block: before broadcast_block");
    let app_network = app.network.clone();
    let block_clone = block.clone();
    tokio::spawn(async move {
        if let Err(e) = app_network.lock().await.broadcast_block(&block_clone) {
            log::error!("broadcast_block error: {}", e);
        }
    });
    info!("create_and_process_block end");
    Ok(())
}

fn calculate_transactions_root(txs: &[Transaction]) -> H256 {
    // Placeholder. A real implementation would build a Merkle Trie.
    if txs.is_empty() {
        return H256::zero();
    }
    let mut hasher = sha3::Keccak256::new();
    for tx in txs {
        hasher.update(tx.hash.as_bytes());
    }
    H256::from_slice(&hasher.finalize())
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

pub async fn main_entry(args: Args) -> Result<()> {
    match args.command {
        Commands::Run(run_args) => run_node(run_args).await?,
        Commands::Init(init_args) => init_node(init_args)?,
        Commands::ChainId => {
            let genesis_path = std::path::Path::new("configs/devnet/genesis.json");
            let genesis: serde_json::Value = fs::read_to_string(genesis_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .ok_or_else(|| anyhow::anyhow!("Failed to read genesis.json for chain_id"))?;
            let chain_id = genesis["config"]["chainId"].as_u64().ok_or_else(|| anyhow::anyhow!("chainId not found in genesis.json"))?;
            println!("0x{:x}", chain_id);
        }
    }
    Ok(())
}
