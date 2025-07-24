use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::proc_macros::rpc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use velora_core::{
    transaction::Transaction,
    types::{Address, BlockNumber, H256, U256},
    Block,
};
use velora_executor::Executor;
use anyhow::{Result, anyhow};
use ethers::types::{Bytes, Transaction as EthersTransaction, TransactionRequest};
use std::str::FromStr;

#[rpc(server, client)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> Result<String>;

    #[method(name = "eth_getBlockByNumber")]
    async fn get_block_by_number(&self, block_number: String, full_tx: bool) -> Result<Option<serde_json::Value>>;

    #[method(name = "eth_getTransactionReceipt")]
    async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<serde_json::Value>>;

    #[method(name = "eth_sendRawTransaction")]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256>;

    #[method(name = "eth_getBalance")]
    async fn get_balance(&self, address: Address, block_number: Option<String>) -> Result<String>;

    #[method(name = "eth_call")]
    async fn call(&self, tx: TransactionRequest, block_number: Option<String>) -> Result<Bytes>;
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

    async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<serde_json::Value>> {
        let receipt = self.ctx.executor.get_receipt(tx_hash)?;
        Ok(receipt.map(|r| serde_json::to_value(r).unwrap()))
    }

    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256> {
        let tx: EthersTransaction = rlp::decode(&bytes)?;
        let velora_tx = Transaction {
            nonce: tx.nonce.as_u64(),
            gas_price: tx.gas_price.unwrap_or_default(),
            gas_limit: tx.gas,
            to: tx.to,
            value: tx.value,
            data: tx.input.to_vec(),
            v: U256::from(tx.v.as_u64()),
            r: tx.r,
            s: tx.s,
            from: Some(tx.from),
            hash: tx.hash,
        };
        let hash = velora_tx.hash;
        self.ctx.tx_sender.send(velora_tx).map_err(|e| anyhow!("Failed to send tx: {}", e))?;
        Ok(hash)
    }

    async fn get_balance(&self, _address: Address, _block_number: Option<String>) -> Result<String> {
        // This requires reading state from the executor, which is complex with revm's DB trait.
        // We'll implement this properly once the state root calculation is in place.
        Ok("0x0".to_string())
    }

    async fn call(&self, _tx: TransactionRequest, _block_number: Option<String>) -> Result<Bytes> {
        // This requires a read-only EVM execution.
        Ok(Bytes::default())
    }
}

pub async fn run_server(ctx: Arc<RpcContext>, addr: SocketAddr) -> Result<(ServerHandle, SocketAddr)> {
    let server = Server::builder().build(addr).await?;
    let addr = server.local_addr()?;
    let handle = server.start(EthApiServer::new(ctx).into_rpc());
    Ok((handle, addr))
}
