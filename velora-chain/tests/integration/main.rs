use velora_cli;
use anyhow::Result;
use std::path::PathBuf;
use std::fs;
use tempfile::tempdir;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use velora_rpc::EthApiClient;
use tokio::time::{sleep, Duration};

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
