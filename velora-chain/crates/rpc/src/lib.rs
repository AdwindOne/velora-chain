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
use revm::primitives::alloy_primitives::{Uint as RevmUint, Address as RevmAddress};
use revm::primitives::{ExecutionResult, Output};
use revm::DatabaseRef;
use velora_core::types::U256;

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

    #[method(name = "eth_getTransactionByHash")]
    async fn get_transaction_by_hash(&self, tx_hash: H256) -> Result<Option<serde_json::Value>, ErrorObjectOwned>;
    #[method(name = "eth_getBlockByHash")]
    async fn get_block_by_hash(&self, block_hash: H256, full_tx: bool) -> Result<Option<serde_json::Value>, ErrorObjectOwned>;
    #[method(name = "eth_chainId")]
    async fn chain_id(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "web3_clientVersion")]
    async fn client_version(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "net_version")]
    async fn net_version(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "eth_gasPrice")]
    async fn gas_price(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "eth_estimateGas")]
    async fn estimate_gas(&self, tx: TransactionRequest, block_number: Option<String>) -> Result<String, ErrorObjectOwned>;
    #[method(name = "eth_accounts")]
    async fn accounts(&self) -> Result<Vec<Address>, ErrorObjectOwned>;
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

fn rpc_err(msg: impl ToString) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(-32000, msg.to_string(), None::<()> )
}

#[async_trait]
impl EthApiServer for VeloraEthApi {
    async fn block_number(&self) -> Result<String, ErrorObjectOwned> {
        let block = self.ctx.executor.get_latest_block().map_err(|e| rpc_err(e.to_string()))?;
        Ok(format!("0x{:x}", block.header.number))
    }
    async fn get_block_by_number(&self, block_number: String, full_tx: bool) -> Result<Option<serde_json::Value>, ErrorObjectOwned> {
        let number = if let Some(stripped) = block_number.strip_prefix("0x") {
            u64::from_str_radix(stripped, 16).unwrap_or(0)
        } else {
            block_number.parse().unwrap_or(0)
        };
        let block_opt = self.ctx.executor.get_block_by_number(number).map_err(|e| rpc_err(e.to_string()))?;
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
        let receipt_opt = self.ctx.executor.get_receipt(tx_hash).map_err(|e| rpc_err(e.to_string()))?;
        if let Some(receipt) = receipt_opt {
            Ok(Some(serde_json::to_value(receipt).unwrap_or(json!({"error": "serialization failed"}))))
        } else {
            Ok(None)
        }
    }
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256, ErrorObjectOwned> {
        // 这里只做 mock 入队，真实实现应解析并校验 tx
        let tx = rlp::decode::<Transaction>(&bytes).map_err(|e| rpc_err(e.to_string()))?;
        self.ctx.tx_sender.send(tx.clone()).map_err(|e| rpc_err(e.to_string()))?;
        Ok(tx.hash)
    }
    async fn get_balance(&self, address: Address, _block_number: Option<String>) -> Result<String, ErrorObjectOwned> {
        let info_opt = self.ctx.executor.as_ref().basic_ref(RevmAddress::from_slice(address.as_bytes())).map_err(|e| rpc_err(e.to_string()))?;
        let balance = info_opt.map(|acc| U256::from_big_endian(&acc.balance.to_be_bytes::<32>())).unwrap_or(U256::zero());
        Ok(format!("0x{:x}", balance))
    }
    async fn call(&self, tx: TransactionRequest, block_number: Option<String>) -> Result<Bytes, ErrorObjectOwned> {
        use revm::primitives::{Env, TxEnv, TransactTo};
        use revm::Evm;

        // 1. 获取区块环境
        let block = if let Some(num_str) = block_number {
            let number = if let Some(stripped) = num_str.strip_prefix("0x") {
                u64::from_str_radix(stripped, 16).unwrap_or(0)
            } else {
                num_str.parse().unwrap_or(0)
            };
            self.ctx.executor.get_block_by_number(number).map_err(|e| rpc_err(e.to_string()))?
        } else {
            Some(self.ctx.executor.get_latest_block().map_err(|e| rpc_err(e.to_string()))?)
        };
        let header = block.as_ref().map(|b| &b.header);

        // 2. 构造 EVM 环境
        let mut env = Env::default();
        if let Some(header) = header {
            // number
            let mut num_bytes = [0u8; 32];
            num_bytes[24..].copy_from_slice(&header.number.to_be_bytes());
            env.block.number = RevmUint::<256, 4>::from_be_bytes(num_bytes);
            // timestamp
            num_bytes = [0u8; 32];
            num_bytes[24..].copy_from_slice(&header.timestamp.to_be_bytes());
            env.block.timestamp = RevmUint::<256, 4>::from_be_bytes(num_bytes);
            // gas_limit
            num_bytes = [0u8; 32];
            num_bytes[24..].copy_from_slice(&header.gas_limit.to_be_bytes());
            env.block.gas_limit = RevmUint::<256, 4>::from_be_bytes(num_bytes);
            env.block.coinbase = RevmAddress::from_slice(header.beneficiary.as_bytes());
        }
        let from = tx.from.unwrap_or(Address::zero());
        let to = match tx.to {
            Some(ethers::types::NameOrAddress::Address(addr)) => addr,
            Some(_) => return Err(rpc_err("Only Address type supported for 'to'")),
            None => Address::zero(),
        };
        // gas_limit 取 u64
        let gas_limit = tx.gas.unwrap_or(U256::from(30_000_000)).as_u64();
        // gas_price, value 转换为 RevmUint<256, 4>
        let mut price_bytes = [0u8; 32];
        let mut value_bytes = [0u8; 32];
        tx.gas_price.unwrap_or(U256::zero()).to_big_endian(&mut price_bytes);
        tx.value.unwrap_or(U256::zero()).to_big_endian(&mut value_bytes);
        env.tx = TxEnv {
            caller: RevmAddress::from_slice(from.as_bytes()),
            gas_limit,
            gas_price: RevmUint::<256, 4>::from_be_bytes(price_bytes),
            transact_to: TransactTo::Call(RevmAddress::from_slice(to.as_bytes())),
            value: RevmUint::<256, 4>::from_be_bytes(value_bytes),
            data: tx.data.unwrap_or_default().to_vec().into(),
            ..Default::default()
        };
        // 用 CacheDB 包裹 Executor
        let mut cache_db = revm::db::CacheDB::new(self.ctx.executor.as_ref());
        let mut evm = Evm::builder().with_db(&mut cache_db).with_env(Box::new(env)).build();
        let result = evm.transact().map_err(|e| rpc_err(e.to_string()))?;
        let output = match result.result {
            ExecutionResult::Success { output, .. } => match output {
                Output::Call(ret) => Bytes::from(ret.to_vec()),
                Output::Create(_, _) => Bytes::from(vec![]),
            },
            _ => Bytes::from(vec![]),
        };
        Ok(output)
    }

    async fn get_transaction_by_hash(&self, tx_hash: H256) -> Result<Option<serde_json::Value>, ErrorObjectOwned> {
        // 遍历所有区块查找交易（可优化为索引）
        let latest = self.ctx.executor.get_latest_block().map_err(|e| rpc_err(e.to_string()))?;
        for n in (0..=latest.header.number).rev() {
            if let Some(block) = self.ctx.executor.get_block_by_number(n).map_err(|e| rpc_err(e.to_string()))? {
                for tx in &block.transactions {
                    if tx.hash == tx_hash {
                        return Ok(Some(serde_json::to_value(tx).unwrap_or(json!({"error": "serialization failed"}))));
                    }
                }
            }
        }
        Ok(None)
    }
    async fn get_block_by_hash(&self, block_hash: H256, full_tx: bool) -> Result<Option<serde_json::Value>, ErrorObjectOwned> {
        let block_opt = self.ctx.executor.get_block_by_hash(block_hash).map_err(|e| rpc_err(e.to_string()))?;
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
    async fn chain_id(&self) -> Result<String, ErrorObjectOwned> {
        // 直接读取 genesis.json
        let genesis_path = std::path::Path::new("configs/devnet/genesis.json");
        let genesis: serde_json::Value = std::fs::read_to_string(genesis_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or_else(|| rpc_err("Failed to read genesis.json for chain_id"))?;
        let chain_id = genesis["config"]["chainId"].as_u64().ok_or_else(|| rpc_err("chainId not found in genesis.json"))?;
        Ok(format!("0x{:x}", chain_id))
    }
    async fn client_version(&self) -> Result<String, ErrorObjectOwned> {
        Ok("velora/0.1.0".to_string())
    }
    async fn net_version(&self) -> Result<String, ErrorObjectOwned> {
        // 直接读取 genesis.json
        let genesis_path = std::path::Path::new("configs/devnet/genesis.json");
        let genesis: serde_json::Value = std::fs::read_to_string(genesis_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or_else(|| rpc_err("Failed to read genesis.json for net_version"))?;
        let chain_id = genesis["config"]["chainId"].as_u64().ok_or_else(|| rpc_err("chainId not found in genesis.json"))?;
        Ok(chain_id.to_string())
    }
    async fn gas_price(&self) -> Result<String, ErrorObjectOwned> {
        // 返回固定 gas price
        Ok(format!("0x{:x}", 1_000_000_000u64)) // 1 gwei
    }
    async fn estimate_gas(&self, _tx: TransactionRequest, _block_number: Option<String>) -> Result<String, ErrorObjectOwned> {
        // 返回固定 gas 21000
        Ok(format!("0x{:x}", 21_000u64))
    }
    async fn accounts(&self) -> Result<Vec<Address>, ErrorObjectOwned> {
        // 不管理账户，返回空数组
        Ok(vec![])
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
