use anyhow::{anyhow, Result};
use bincode::{deserialize, serialize};
use ethers::types::Log as EthersLog;
use ethers::types::{Bytes as EthersBytes, H160};
use revm::{
    db::{CacheDB, DatabaseRef},
    primitives::alloy_primitives::{Uint, Address as AlloyAddress, B256},
    primitives::{
        AccountInfo, Address as RevmAddress, BlockEnv, Bytecode, CfgEnv, CreateScheme, Env,
        ExecutionResult, Log, Output, TransactTo, TxEnv,
    },
};
use rocksdb::{Options, TransactionDB, TransactionDBOptions, WriteBatchWithTransaction};
use std::path::Path;
use std::sync::Arc;
use velora_core::{
    block::{Block, BlockHeader},
    genesis::Genesis,
    receipt::Receipt,
    transaction::Transaction,
    types::{Bloom, H256, U256},
};
use zerocopy::IntoBytes;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct AccountInfoSerde {
    pub balance: [u8; 32],
    pub nonce: u64,
    pub code_hash: [u8; 32],
    pub code: Option<Vec<u8>>,
}

impl From<&AccountInfo> for AccountInfoSerde {
    fn from(info: &AccountInfo) -> Self {
        Self {
            balance: info.balance.to_be_bytes::<32>(),
            nonce: info.nonce,
            code_hash: info.code_hash.0,
            code: info.code.as_ref().map(|c| c.bytes().to_vec()),
        }
    }
}

impl From<AccountInfoSerde> for AccountInfo {
    fn from(info: AccountInfoSerde) -> Self {
        Self {
            balance: Uint::<256, 4>::from_be_bytes(info.balance),
            nonce: info.nonce,
            code_hash: B256::new(info.code_hash),
            code: info.code.map(|v| Bytecode::new_raw(v.into())),
        }
    }
}

const LATEST_BLOCK_KEY: &[u8] = b"latest_block";
const BLOCK_NUMBER_KEY_PREFIX: &[u8] = b"n:";
const BLOCK_HASH_KEY_PREFIX: &[u8] = b"h:";
const RECEIPTS_KEY_PREFIX: &[u8] = b"r:";
const ACCOUNTS_CF: &str = "accounts";
const CODE_CF: &str = "code";
const STORAGE_CF: &str = "storage";
const RECEIPTS_CF: &str = "receipts";

pub struct Executor {
    db: Arc<TransactionDB>,
}

impl Executor {
    pub fn new(path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = [ACCOUNTS_CF, CODE_CF, STORAGE_CF, RECEIPTS_CF];
        let tx_db_opts = TransactionDBOptions::default();
        let db = Arc::new(TransactionDB::open_cf(&opts, &tx_db_opts, path, cfs)?);

        Ok(Self { db })
    }

    pub fn get_latest_block(&self) -> Result<Block> {
        let block_bytes = self
            .db
            .get(LATEST_BLOCK_KEY)?
            .ok_or_else(|| anyhow!("Latest block not found"))?;
        Ok(deserialize(&block_bytes)?)
    }

    pub fn get_block_by_number(&self, number: u64) -> Result<Option<Block>> {
        let key = [BLOCK_NUMBER_KEY_PREFIX, &number.to_be_bytes()].concat();
        let hash_bytes_opt = self.db.get(key)?;
        if let Some(hash_bytes) = hash_bytes_opt {
            let hash = H256::from_slice(&hash_bytes);
            self.get_block_by_hash(hash)
        } else {
            Ok(None)
        }
    }

    pub fn get_block_by_hash(&self, hash: H256) -> Result<Option<Block>> {
        let key = [BLOCK_HASH_KEY_PREFIX, hash.as_bytes()].concat();
        match self.db.get(key)? {
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn get_receipt(&self, tx_hash: H256) -> Result<Option<Receipt>> {
        let receipts_cf = self.db.cf_handle(RECEIPTS_CF).unwrap();
        let key = [RECEIPTS_KEY_PREFIX, tx_hash.as_bytes()].concat();
        match self.db.get_cf(receipts_cf, key)? {
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn apply_block(&self, block: &Block) -> Result<()> {
        let _parent = self
            .get_block_by_hash(block.header.parent_hash)?
            .ok_or_else(|| anyhow!("Parent block not found"))?;
        let mut db = CacheDB::new(self);
        let mut cumulative_gas_used = U256::zero();
        let mut receipts = Vec::new();

        for (tx_index, tx) in block.transactions.iter().enumerate() {
            let mut evm = revm::Evm::builder()
                .with_db(&mut db)
                .with_env(Box::new(self.build_env(&block.header, tx)))
                .build();

            let result = evm.transact_commit().map_err(|e| anyhow!(e.to_string()))?;
            let gas_used = result.gas_used();
            cumulative_gas_used += U256::from(gas_used);

            let (status, contract_address) = match &result {
                ExecutionResult::Success { output, .. } => {
                    let addr = if let Output::Create(_, Some(addr)) = output {
                        Some(*addr)
                    } else {
                        None
                    };
                    (1, addr)
                }
                ExecutionResult::Revert { .. } => (0, None),
                ExecutionResult::Halt { .. } => (0, None),
            };

            let logs = result
                .logs()
                .iter()
                .map(|log| convert_log((*log).clone()))
                .collect::<Vec<_>>();
            let logs_bloom = logs.iter().fold(Bloom::default(), |mut bloom, log| {
                use ethbloom::Input;
                bloom.accrue(Input::Raw(log.address.as_bytes()));
                bloom
            });

            let receipt = Receipt {
                tx_hash: tx.hash,
                tx_index,
                block_hash: block.hash(),
                block_number: block.header.number,
                from: tx.from.unwrap_or_default(),
                to: tx.to,
                cumulative_gas_used,
                gas_used: Some(U256::from(gas_used)),
                contract_address: contract_address.map(|addr| H160::from_slice(addr.as_bytes())),
                logs,
                status,
                logs_bloom,
                state_root: None, // Only for post-byzantium blocks
            };
            receipts.push(receipt);
        }

        let mut batch = WriteBatchWithTransaction::<true>::default();
        let block_bytes = serialize(block)?;
        batch.put(LATEST_BLOCK_KEY, &block_bytes);
        batch.put(
            [BLOCK_NUMBER_KEY_PREFIX, &block.header.number.to_be_bytes()].concat(),
            block.hash().as_bytes(),
        );
        batch.put(
            [BLOCK_HASH_KEY_PREFIX, block.hash().as_bytes()].concat(),
            &block_bytes,
        );

        let receipts_cf = self.db.cf_handle(RECEIPTS_CF).unwrap();
        for receipt in receipts {
            let key = [RECEIPTS_KEY_PREFIX, receipt.tx_hash.as_bytes()].concat();
            let receipt_bytes = serialize(&receipt)?;
            batch.put_cf(receipts_cf, key, &receipt_bytes);
        }

        for (addr, acc) in db.accounts {
            if !acc.info.is_empty() {
                let info_serde = AccountInfoSerde::from(&acc.info);
                let acc_bytes = serialize(&info_serde)?;
                let cf = self.db.cf_handle(ACCOUNTS_CF).unwrap();
                batch.put_cf(cf, addr.as_bytes(), &acc_bytes);
            }
        }

        self.db.write(batch)?;
        Ok(())
    }

    pub fn apply_genesis(&self, genesis: &Genesis) -> Result<()> {
        let accounts_cf = self.db.cf_handle(ACCOUNTS_CF).unwrap();
        for (addr, acc) in &genesis.alloc {
            let account_info = AccountInfo {
                balance: u256_to_revm_u256(acc.balance),
                nonce: 0,
                code_hash: B256::new([0u8; 32]),
                code: None,
            };
            let info_serde = AccountInfoSerde::from(&account_info);
            self.db
                .put_cf(accounts_cf, addr.as_bytes(), serialize(&info_serde)?)?;
        }

        let genesis_block = Block {
            header: BlockHeader {
                number: 0,
                timestamp: genesis.timestamp,
                ..Default::default()
            },
            transactions: vec![],
            ommers: vec![],
        };

        let block_bytes = serialize(&genesis_block)?;
        self.db.put(LATEST_BLOCK_KEY, &block_bytes)?;
        self.db.put(
            [BLOCK_NUMBER_KEY_PREFIX, &0u64.to_be_bytes()].concat(),
            genesis_block.hash().as_bytes(),
        )?;
        self.db.put(
            [BLOCK_HASH_KEY_PREFIX, genesis_block.hash().as_bytes()].concat(),
            &block_bytes,
        )?;

        Ok(())
    }

    fn build_env(&self, header: &BlockHeader, tx: &Transaction) -> Env {
        Env {
            cfg: CfgEnv::default(),
            block: BlockEnv {
                number: u256_to_revm_u256(header.number.into()),
                coinbase: RevmAddress(AlloyAddress::from_slice(header.beneficiary.as_bytes()).0),
                timestamp: u256_to_revm_u256(header.timestamp.into()),
                gas_limit: u256_to_revm_u256(header.gas_limit.into()),
                ..Default::default()
            },
            tx: TxEnv {
                caller: RevmAddress(AlloyAddress::from_slice(tx.from.unwrap_or_default().as_bytes()).0),
                gas_limit: tx.gas_limit.as_u64(),
                gas_price: u256_to_revm_u256(tx.gas_price),
                transact_to: tx.to.map_or_else(
                    || TransactTo::Create(CreateScheme::Create),
                    |addr| TransactTo::Call(RevmAddress(AlloyAddress::from_slice(addr.as_bytes()).0)),
                ),
                value: u256_to_revm_u256(tx.value),
                data: tx.data.clone().into(),
                nonce: Some(tx.nonce),
                ..Default::default()
            },
        }
    }
}

fn convert_log(log: Log) -> EthersLog {
    EthersLog {
        address: H160::from_slice(log.address.as_bytes()),
        topics: log
            .topics()
            .iter()
            .map(|b| H256::from_slice(b.as_bytes()))
            .collect(),
        data: EthersBytes::from(log.data.data.to_vec()),
        block_hash: None,
        block_number: None,
        transaction_hash: None,
        transaction_index: None,
        log_index: None,
        transaction_log_index: None,
        log_type: None,
        removed: None,
    }
}

fn u256_to_revm_u256(val: U256) -> Uint<256, 4> {
    let mut bytes = [0u8; 32];
    val.to_big_endian(&mut bytes);
    Uint::<256, 4>::from_be_bytes(bytes)
}
#[allow(dead_code)]
fn revm_u256_to_u256(val: Uint<256, 4>) -> U256 {
    U256::from_big_endian(&val.to_be_bytes::<32>())
}

impl AsRef<Executor> for Executor {
    fn as_ref(&self) -> &Executor {
        self
    }
}

impl DatabaseRef for Executor {
    type Error = anyhow::Error;
    fn basic_ref(&self, address: RevmAddress) -> Result<Option<AccountInfo>, Self::Error> {
        let accounts_cf = self.db.cf_handle(ACCOUNTS_CF).unwrap();
        match self.db.get_cf(accounts_cf, address.as_bytes())? {
            Some(bytes) => Ok(Some(AccountInfo::from(deserialize::<AccountInfoSerde>(
                &bytes,
            )?))),
            None => Ok(None),
        }
    }
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let code_cf = self.db.cf_handle(CODE_CF).unwrap();
        match self.db.get_cf(code_cf, code_hash.as_bytes())? {
            Some(bytes) => Ok(Bytecode::new_raw(bytes.into())),
            None => Ok(Bytecode::default()),
        }
    }
    fn storage_ref(
        &self,
        address: RevmAddress,
        index: Uint<256, 4>,
    ) -> Result<Uint<256, 4>, Self::Error> {
        let storage_cf = self.db.cf_handle(STORAGE_CF).unwrap();
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(address.as_bytes());
        key[32..].copy_from_slice(&index.to_be_bytes::<32>());
        match self.db.get_cf(storage_cf, key)? {
            Some(bytes) => Ok(Uint::<256, 4>::from_be_bytes::<32>(
                bytes.try_into().unwrap(),
            )),
            None => Ok(Uint::<256, 4>::ZERO),
        }
    }
    fn block_hash_ref(&self, number: Uint<256, 4>) -> Result<B256, Self::Error> {
        let num_u64 = number.to::<u64>();
        let key = [BLOCK_NUMBER_KEY_PREFIX, &num_u64.to_be_bytes()].concat();
        match self.db.get(key)? {
            Some(bytes) => Ok(B256::from_slice(&bytes)),
            None => Ok(B256::ZERO),
        }
    }
}
