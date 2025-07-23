use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::proc_macros::rpc;
use std::net::SocketAddr;
use velora_core::types::{BlockNumber, H256};
use anyhow::Result;

#[rpc(server)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> Result<String>;

    #[method(name = "eth_getBlockByNumber")]
    async fn get_block_by_number(&self, block_number: String, full_tx: bool) -> Result<Option<serde_json::Value>>;
}

pub struct EthApiServer;

#[async_trait::async_trait]
impl EthApiServer for EthApiServer {
    async fn block_number(&self) -> Result<String> {
        // In a real implementation, we would get the block number from the database
        Ok("0x0".to_string())
    }

    async fn get_block_by_number(&self, block_number: String, full_tx: bool) -> Result<Option<serde_json::Value>> {
        // In a real implementation, we would get the block from the database
        Ok(None)
    }
}

pub async fn run_server(addr: SocketAddr) -> Result<(ServerHandle, SocketAddr)> {
    let server = Server::builder().build(addr).await?;
    let addr = server.local_addr()?;
    let handle = server.start(EthApiServer.into_rpc());
    Ok((handle, addr))
}
