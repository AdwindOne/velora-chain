use velora_cli;
use anyhow::Result;
use std::path::PathBuf;
use std::fs;
use tempfile::tempdir;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use velora_rpc::EthApiClient;
use tokio::time::{sleep, Duration};

use ethers::{
    core::rand::thread_rng,
    signers::{LocalWallet, Signer},
    types::{TransactionRequest, Bytes},
};

#[tokio::test]
async fn single_node_produces_blocks() -> Result<()> {
    let temp_dir = tempdir()?;
    let datadir = temp_dir.path().to_path_buf();
    let genesis_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configs/devnet/genesis.json");

    // 1. Initialize the node
    let init_args = velora_cli::InitArgs {
        datadir: datadir.clone(),
        genesis: genesis_path,
    };
    velora_cli::init_node(init_args)?;

    // 2. Run the node in the background
    let run_args = velora_cli::RunArgs {
        datadir,
        p2p_port: 30309, // Use a unique port for testing
        rpc_addr: "127.0.0.1:8549".parse()?,
        block_time: 1000, // 1 second block time
    };

    tokio::spawn(async move {
        velora_cli::run_node(run_args).await.unwrap();
    });

    // Give the node time to start up
    sleep(Duration::from_secs(2)).await;

    // 3. Connect an RPC client
    let rpc_url = "http://127.0.0.1:8549";
    let client = HttpClientBuilder::default().build(rpc_url)?;

    // 4. Check that block number increases
    let start_block_str = client.block_number().await?;
    let start_block = u64::from_str_radix(&start_block_str[2..], 16)?;

    sleep(Duration::from_secs(3)).await;

    let end_block_str = client.block_number().await?;
    let end_block = u64::from_str_radix(&end_block_str[2..], 16)?;

    assert!(end_block > start_block, "Block number should increase");

    // 5. Get a block and verify its contents
    let block = client.get_block_by_number(end_block_str, false).await?.unwrap();
    assert_eq!(block["number"], format!("0x{:x}", end_block));

    Ok(())
}

#[tokio::test]
async fn single_node_processes_transaction() -> Result<()> {
    let temp_dir = tempdir()?;
    let datadir = temp_dir.path().to_path_buf();
    let genesis_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configs/devnet/genesis.json");

    // 1. Initialize the node
    let init_args = velora_cli::InitArgs {
        datadir: datadir.clone(),
        genesis: genesis_path,
    };
    velora_cli::init_node(init_args)?;

    // 2. Run the node in the background
    let run_args = velora_cli::RunArgs {
        datadir,
        p2p_port: 30310, // Use a unique port for testing
        rpc_addr: "127.0.0.1:8550".parse()?,
        block_time: 2000, // 2 second block time
    };

    tokio::spawn(async move {
        velora_cli::run_node(run_args).await.unwrap();
    });

    // Give the node time to start up
    sleep(Duration::from_secs(2)).await;

    // 3. Connect an RPC client
    let rpc_url = "http://127.0.0.1:8550";
    let client = HttpClientBuilder::default().build(rpc_url)?;

    // 4. Create a transaction
    let wallet = LocalWallet::new(&mut thread_rng());
    let to_addr = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8".parse()?;
    let tx = TransactionRequest::new()
        .to(to_addr)
        .value(1000)
        .from(wallet.address());

    let signature = wallet.sign_transaction(&tx.clone().into()).await?;
    let raw_tx = tx.rlp_signed(&signature);

    // 5. Send the transaction
    let tx_hash = client.send_raw_transaction(raw_tx).await?;
    assert_ne!(tx_hash, velora_core::H256::zero());

    // 6. Wait for the transaction to be mined
    let mut receipt = None;
    for _ in 0..10 {
        receipt = client.get_transaction_receipt(tx_hash).await?;
        if receipt.is_some() {
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    let receipt = receipt.expect("Transaction should be mined");
    assert_eq!(receipt["transactionHash"], tx_hash);
    assert_eq!(receipt["status"], "0x1");

    Ok(())
}
