use ethers::types::{Address, Bytes, TransactionRequest, H256};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::types::ErrorObjectOwned;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use velora_core::transaction::Transaction;
use velora_executor::Executor;
use async_trait::async_trait;
use serde_json::json;

#[rpc(server, client)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "eth_getBlockByNumber")]
    async fn get_block_by_number(
        &self,
        block_number: String,
        full_tx: bool,
    ) -> Result<Option<serde_json::Value>, ErrorObjectOwned>;

    #[method(name = "eth_getTransactionReceipt")]
    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<serde_json::Value>, ErrorObjectOwned>;

    #[method(name = "eth_sendRawTransaction")]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256, ErrorObjectOwned>;

    #[method(name = "eth_getBalance")]
    async fn get_balance(
        &self,
        address: Address,
        block_number: Option<String>,
    ) -> Result<String, ErrorObjectOwned>;

    #[method(name = "eth_call")]
    async fn call(
        &self,
        tx: TransactionRequest,
        block_number: Option<String>,
    ) -> Result<Bytes, ErrorObjectOwned>;
}

pub struct RpcContext {
    pub executor: Arc<Executor>,
    pub tx_sender: mpsc::UnboundedSender<Transaction>,
}

pub struct VeloraEthApi {
    ctx: Arc<RpcContext>,
}

impl VeloraEthApi {
    pub fn new(ctx: Arc<RpcContext>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl EthApiServer for VeloraEthApi {
    async fn block_number(&self) -> Result<String, ErrorObjectOwned> {
        let block = self.ctx.executor.get_latest_block().map_err(|e| ErrorObjectOwned::from(e.to_string()))?;
        Ok(format!("0x{:x}", block.header.number))
    }
    async fn get_block_by_number(&self, block_number: String, full_tx: bool) -> Result<Option<serde_json::Value>, ErrorObjectOwned> {
        let number = if let Some(stripped) = block_number.strip_prefix("0x") {
            u64::from_str_radix(stripped, 16).unwrap_or(0)
        } else {
            block_number.parse().unwrap_or(0)
        };
        let block_opt = self.ctx.executor.get_block_by_number(number).map_err(|e| ErrorObjectOwned::from(e.to_string()))?;
        if let Some(block) = block_opt {
            let value = if full_tx {
                serde_json::to_value(&block).unwrap_or(json!({"error": "serialization failed"}))
            } else {
                let mut block_json = serde_json::to_value(&block).unwrap_or(json!({}));
                if let Some(obj) = block_json.as_object_mut() {
                    obj.insert("transactions".to_string(), json!(block.transactions.iter().map(|tx| format!("0x{:x}", tx.hash)).collect::<Vec<_>>()));
                }
                block_json
            };
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<serde_json::Value>, ErrorObjectOwned> {
        let receipt_opt = self.ctx.executor.get_receipt(tx_hash).map_err(|e| ErrorObjectOwned::from(e.to_string()))?;
        if let Some(receipt) = receipt_opt {
            Ok(Some(serde_json::to_value(receipt).unwrap_or(json!({"error": "serialization failed"}))))
        } else {
            Ok(None)
        }
    }
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256, ErrorObjectOwned> {
        // 这里只做 mock 入队，真实实现应解析并校验 tx
        let tx = rlp::decode::<Transaction>(&bytes).map_err(|e| ErrorObjectOwned::from(e.to_string()))?;
        self.ctx.tx_sender.send(tx.clone()).map_err(|e| ErrorObjectOwned::from(e.to_string()))?;
        Ok(tx.hash)
    }
    async fn get_balance(&self, address: Address, _block_number: Option<String>) -> Result<String, ErrorObjectOwned> {
        use velora_core::types::U256;
        let info_opt = self.ctx.executor.basic_ref(address.into()).map_err(|e| ErrorObjectOwned::from(e.to_string()))?;
        let balance = info_opt.map(|acc| U256::from_big_endian(&acc.balance.to_be_bytes::<32>())).unwrap_or(U256::zero());
        Ok(format!("0x{:x}", balance))
    }
    async fn call(&self, _tx: TransactionRequest, _block_number: Option<String>) -> Result<Bytes, ErrorObjectOwned> {
        // 这里只做 mock，真实实现应执行只读 EVM 调用
        Ok(Bytes::from(vec![]))
    }
}

pub async fn run_server(
    ctx: Arc<RpcContext>,
    addr: SocketAddr,
) -> Result<(ServerHandle, SocketAddr), Box<dyn std::error::Error + Send + Sync>> {
    let server = Server::builder().build(addr).await?;
    let local_addr = server.local_addr()?;
    let eth_api = VeloraEthApi::new(ctx);
    let handle = server.start(eth_api.into_rpc());
    Ok((handle, local_addr))
}
