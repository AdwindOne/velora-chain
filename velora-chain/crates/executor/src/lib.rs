use velora_core::{
    account::Account,
    block::{Block, BlockHeader},
    genesis::Genesis,
    transaction::Transaction,
    types::{Address, H256, U256},
};
use revm::{
    db::{CacheDB, DatabaseRef, EmptyDB},
    primitives::{
        AccountInfo, Bytecode, CfgEnv, Env, ExecutionResult, Output, TransactTo, TxEnv,
        B160, B256, U256 as RevmU256,
    },
    Database,
};
use rocksdb::{TransactionDB, TransactionDBOptions, Options, WriteBatch};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::path::Path;
use bincode::{serialize, deserialize};
use log::{info, warn};

const LATEST_BLOCK_KEY: &[u8] = b"latest_block";
const BLOCK_NUMBER_KEY_PREFIX: &[u8] = b"n:";
const BLOCK_HASH_KEY_PREFIX: &[u8] = b"h:";
const ACCOUNTS_CF: &str = "accounts";
const CODE_CF: &str = "code";
const STORAGE_CF: &str = "storage";

pub struct Executor {
    db: Arc<TransactionDB>,
}

impl Executor {
    pub fn new(path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = [ACCOUNTS_CF, CODE_CF, STORAGE_CF];
        let tx_db_opts = TransactionDBOptions::default();
        let db = Arc::new(TransactionDB::open_cf(&opts, &tx_db_opts, path, &cfs)?);

        Ok(Self { db })
    }

    pub fn get_latest_block(&self) -> Result<Block> {
        let block_bytes = self.db.get(LATEST_BLOCK_KEY)?.ok_or_else(|| anyhow!("Latest block not found"))?;
        Ok(deserialize(&block_bytes)?)
    }

    pub fn get_block_by_number(&self, number: u64) -> Result<Option<Block>> {
        let key = [BLOCK_NUMBER_KEY_PREFIX, &number.to_be_bytes()].concat();
        let hash_bytes = self.db.get(key)?.ok_or_else(|| anyhow!("Block hash for number {} not found", number))?;
        let hash = H256::from_slice(&hash_bytes);
        self.get_block_by_hash(hash)
    }

    pub fn get_block_by_hash(&self, hash: H256) -> Result<Option<Block>> {
        let key = [BLOCK_HASH_KEY_PREFIX, hash.as_bytes()].concat();
        match self.db.get(key)? {
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn apply_block(&self, block: &Block) -> Result<()> {
        let parent = self.get_block_by_hash(block.header.parent_hash)?.ok_or_else(|| anyhow!("Parent block not found"))?;
        let mut db = CacheDB::new(self.as_ref());

        for tx in &block.transactions {
            let mut evm = revm::EVM::new();
            evm.database(&mut db);
            evm.env = self.build_env(&block.header, tx);

            let result = evm.transact_commit()?;
            // We should generate and store receipts here
        }

        let mut batch = WriteBatch::default();
        let block_bytes = serialize(block)?;
        batch.put(LATEST_BLOCK_KEY, &block_bytes);
        batch.put([BLOCK_NUMBER_KEY_PREFIX, &block.header.number.to_be_bytes()].concat(), block.hash().as_bytes());
        batch.put([BLOCK_HASH_KEY_PREFIX, block.hash().as_bytes()].concat(), &block_bytes);

        // Commit state changes from CacheDB to RocksDB
        for (addr, acc) in db.accounts {
            if acc.is_touched() {
                let info = acc.info.clone().unwrap_or_default();
                let acc_bytes = serialize(&info)?;
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
                balance: acc.balance.into(),
                nonce: 0,
                code_hash: B256::zero(),
                code: None,
            };
            self.db.put_cf(accounts_cf, addr.as_bytes(), serialize(&account_info)?)?;
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
        self.db.put([BLOCK_NUMBER_KEY_PREFIX, &0u64.to_be_bytes()].concat(), genesis_block.hash().as_bytes())?;
        self.db.put([BLOCK_HASH_KEY_PREFIX, genesis_block.hash().as_bytes()].concat(), &block_bytes)?;

        Ok(())
    }

    fn build_env(&self, header: &BlockHeader, tx: &Transaction) -> Env {
        Env {
            cfg: CfgEnv::default(),
            block: revm::primitives::BlockEnv {
                number: RevmU256::from(header.number),
                coinbase: header.beneficiary.into(),
                timestamp: RevmU256::from(header.timestamp),
                gas_limit: RevmU256::from(header.gas_limit),
                ..Default::default()
            },
            tx: TxEnv {
                caller: tx.from.unwrap_or_default().into(),
                gas_limit: tx.gas_limit.as_u64(),
                gas_price: tx.gas_price.into(),
                transact_to: tx.to.map_or(TransactTo::Create, |addr| TransactTo::Call(addr.into())),
                value: tx.value.into(),
                data: tx.data.clone().into(),
                nonce: Some(tx.nonce),
                ..Default::default()
            },
        }
    }
}

impl AsRef<Executor> for Executor {
    fn as_ref(&self) -> &Executor {
        self
    }
}

impl DatabaseRef for Executor {
    type Error = anyhow::Error;
    fn basic_ref(&self, address: B160) -> Result<Option<AccountInfo>, Self::Error> {
        let accounts_cf = self.db.cf_handle(ACCOUNTS_CF).unwrap();
        match self.db.get_cf(accounts_cf, address.as_bytes())? {
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
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
    fn storage_ref(&self, address: B160, index: RevmU256) -> Result<RevmU256, Self::Error> {
        let storage_cf = self.db.cf_handle(STORAGE_CF).unwrap();
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(address.as_bytes());
        key[32..].copy_from_slice(&index.to_be_bytes::<32>());
        match self.db.get_cf(storage_cf, &key)? {
            Some(bytes) => Ok(RevmU256::from_be_bytes(bytes.try_into().unwrap())),
            None => Ok(RevmU256::ZERO),
        }
    }
    fn block_hash_ref(&self, number: RevmU256) -> Result<B256, Self::Error> {
        let num_u64 = number.to::<u64>();
        let key = [BLOCK_NUMBER_KEY_PREFIX, &num_u64.to_be_bytes()].concat();
        match self.db.get(key)? {
            Some(bytes) => Ok(B256::from_slice(&bytes)),
            None => Ok(B256::ZERO),
        }
    }
}
