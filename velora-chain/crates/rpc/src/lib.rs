use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::proc_macros::rpc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use velora_core::types::{BlockNumber, H256};
use velora_core::{Block, Transaction};
use velora_executor::Executor;
use anyhow::{Result, anyhow};
use ethers::types::Bytes;
use std::str::FromStr;

#[rpc(server, client)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> Result<String>;

    #[method(name = "eth_getBlockByNumber")]
    async fn get_block_by_number(&self, block_number: String, full_tx: bool) -> Result<Option<serde_json::Value>>;

    #[method(name = "eth_sendRawTransaction")]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256>;
}

pub struct RpcContext {
    pub executor: Arc<Executor>,
    pub tx_sender: mpsc::UnboundedSender<Transaction>,
}

pub struct EthApiServer {
    ctx: Arc<RpcContext>,
}

impl EthApiServer {
    pub fn new(ctx: Arc<RpcContext>) -> Self {
        Self { ctx }
    }
}

#[async_trait::async_trait]
impl EthApi for EthApiServer {
    async fn block_number(&self) -> Result<String> {
        let block = self.ctx.executor.get_latest_block()?;
        Ok(format!("0x{:x}", block.header.number))
    }

    async fn get_block_by_number(&self, block_number_str: String, _full_tx: bool) -> Result<Option<serde_json::Value>> {
        let block_number = match block_number_str.as_str() {
            "latest" => self.ctx.executor.get_latest_block()?.header.number,
            _ => u64::from_str_radix(&block_number_str[2..], 16)?,
        };

        let block = self.ctx.executor.get_block_by_number(block_number)?;
        Ok(block.map(|b| serde_json::to_value(b).unwrap()))
    }

    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256> {
        // In a real implementation, we would decode the raw tx, validate it,
        // and then send it to the transaction pool.
        // For now, we'll just create a dummy transaction.
        let tx: Transaction = rlp::decode(&bytes)?;
        let hash = tx.hash;
        self.ctx.tx_sender.send(tx).map_err(|e| anyhow!("Failed to send tx: {}", e))?;
        Ok(hash)
    }
}

pub async fn run_server(ctx: Arc<RpcContext>, addr: SocketAddr) -> Result<(ServerHandle, SocketAddr)> {
    let server = Server::builder().build(addr).await?;
    let addr = server.local_addr()?;
    let handle = server.start(EthApiServer::new(ctx).into_rpc());
    Ok((handle, addr))
}
