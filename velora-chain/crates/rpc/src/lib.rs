use ethers::types::{Address, Bytes, TransactionRequest, H256};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::types::ErrorObjectOwned;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use velora_core::transaction::Transaction;
use velora_executor::Executor;

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

pub async fn run_server(
    _ctx: Arc<RpcContext>,
    _addr: SocketAddr,
) -> Result<(ServerHandle, SocketAddr), Box<dyn std::error::Error + Send + Sync>> {
    Server::builder().build(_addr).await?;
    // 这里 EthApiServer 由宏自动生成，需实现 EthApiServer trait 的类型
    // 你需要实现一个结构体并实现 EthApiServer trait，这里暂时留空
    // let handle = server.start(YourEthApiServerImpl::new(ctx).into_rpc());
    unimplemented!("You need to provide an implementation of EthApiServer trait");
    // Ok((handle, addr))
}
